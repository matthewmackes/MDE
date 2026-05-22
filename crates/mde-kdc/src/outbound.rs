//! KDC2-3.6 — shared outbound packet queue.
//!
//! The D-Bus `RingDevice` / `SendSms` / `SendClipboard` /
//! `SendFile` methods turn an operator action into a
//! [`mde_kdc_proto::wire::Packet`] addressed to a paired peer
//! and enqueue it here. A future network worker
//! (KDC2-3.2.a + KDC2-4.4) drains the queue, picks a transport
//! via the mesh-router, and writes the packet on the chosen
//! TLS socket.
//!
//! The queue is intentionally simple — a `Mutex<Vec<...>>` —
//! because the throughput target is operator-scale (clicks per
//! minute) not packet-scale (clicks per second). Replacing with
//! a `tokio::sync::mpsc` channel when we wire the network
//! worker is a one-file change.

use std::sync::{Arc, Mutex};

use mde_kdc_proto::wire::Packet;

/// One pending outbound send. The dispatcher resolves
/// `device_id` to a current transport via the mesh-router; the
/// packet is the already-serialized body.
#[derive(Debug, Clone, PartialEq)]
pub struct OutboundSend {
    /// Paired-device id (KDC UUID). Used by the network worker
    /// to pick the per-peer transport.
    pub device_id: String,
    /// Already-built `Packet` (type-tagged + body-serialized).
    pub packet: Packet,
}

/// Shared queue handle. Cloneable cheaply via `Arc`. Both the
/// D-Bus producer side and the future network-worker drain side
/// hold one of these.
#[derive(Debug, Clone, Default)]
pub struct PendingSends {
    inner: Arc<Mutex<Vec<OutboundSend>>>,
}

impl PendingSends {
    /// Empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue one outbound send. Cheap — single mutex op.
    pub fn push(&self, send: OutboundSend) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.push(send);
        }
    }

    /// Drain every pending send. Used by the network worker on
    /// each tick. Returns the items in FIFO order.
    pub fn drain(&self) -> Vec<OutboundSend> {
        if let Ok(mut guard) = self.inner.lock() {
            std::mem::take(&mut *guard)
        } else {
            Vec::new()
        }
    }

    /// Current backlog length. O(1).
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// True when the queue holds zero sends.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet(kind: &str) -> Packet {
        Packet {
            id: 1,
            kind: kind.to_string(),
            body: serde_json::json!({}),
            ..Default::default()
        }
    }

    #[test]
    fn pending_sends_starts_empty() {
        let q = PendingSends::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn push_and_drain_round_trip_in_fifo_order() {
        let q = PendingSends::new();
        q.push(OutboundSend {
            device_id: "phone-A".into(),
            packet: make_packet("kdeconnect.ping"),
        });
        q.push(OutboundSend {
            device_id: "phone-B".into(),
            packet: make_packet("kdeconnect.clipboard"),
        });
        assert_eq!(q.len(), 2);
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].device_id, "phone-A");
        assert_eq!(drained[1].device_id, "phone-B");
        // Drain resets the queue.
        assert!(q.is_empty());
    }

    #[test]
    fn clone_shares_underlying_state() {
        // Lock: cloning the handle must NOT fork the storage,
        // so the network worker's drain sees the D-Bus
        // producer's pushes.
        let producer = PendingSends::new();
        let consumer = producer.clone();
        producer.push(OutboundSend {
            device_id: "x".into(),
            packet: make_packet("kdeconnect.ping"),
        });
        assert_eq!(consumer.len(), 1);
        let drained = consumer.drain();
        assert_eq!(drained.len(), 1);
        // Producer's view is also empty now (shared storage).
        assert!(producer.is_empty());
    }
}
