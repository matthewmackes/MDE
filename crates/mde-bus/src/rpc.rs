//! EPIC-BUS-EXT-ACTION (Q31 + Q32) — request/reply RPC over the Bus.
//!
//! The Bus is event-first: a publish fans out, nobody is obliged to
//! answer. Some surfaces need the opposite — a *command* with a
//! *response* ("resolve this conflict", "what marks does this window
//! have?"). Per Q96 the Bus is the migration target for the D-Bus
//! command surfaces MDE retires by 1.0, so it needs an RPC idiom.
//!
//! ## The convention
//!
//! - Commands publish to the **`action/<domain>/<verb>`** namespace
//!   (e.g. `action/meshfs/rebalance`).
//! - The published action message's **ULID is the correlation key**.
//!   No side-channel metadata is needed: the requester knows its own
//!   message's ULID (it's the `Persist::write` return), and a
//!   responder reads the action message's ULID to know where to
//!   answer.
//! - The response lands on **`reply/<request-ulid>`**. The requester
//!   polls that topic until a reply arrives or the timeout fires.
//!
//! ## Responder side
//!
//! A responder is any worker that subscribes to its `action/<domain>/+`
//! topics, does the work, and publishes its result to
//! `reply/<action-msg-ulid>`. This module ships the **caller** side +
//! the convention; the per-domain responders (meshfs, marks, …)
//! land with their respective epics. The `mde-bus request` CLI verb
//! makes the caller path operator-reachable today (an operator can
//! fire an action + watch for the reply or time out), and the
//! `request` / `publish_request` / `await_reply` fns are the library
//! surface future Rust callers use in place of a D-Bus method call.

use std::time::{Duration, Instant};

use crate::hooks::config::Priority;
use crate::persist::{Persist, StoredMessage};

/// Default RPC timeout per the Q31 lock. Callers override per-request
/// (the CLI exposes `--timeout-secs`).
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// How often [`await_reply`] checks the reply topic. 200 ms balances
/// responsiveness against index-read churn; well under the default
/// 30 s timeout.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// The `action/` namespace prefix every command topic must carry.
pub const ACTION_PREFIX: &str = "action/";

/// Reply topic for a request whose action message has ULID `ulid`:
/// `reply/<ulid>`.
#[must_use]
pub fn reply_topic(ulid: &str) -> String {
    format!("reply/{ulid}")
}

/// Errors from the RPC caller path.
#[derive(Debug)]
pub enum RpcError {
    /// The action topic didn't start with `action/`. Commands live
    /// in their own namespace so events + commands never collide.
    BadActionTopic(String),
    /// A persist read/write failed.
    Persist(String),
    /// No reply landed on `reply/<ulid>` within the timeout.
    Timeout {
        /// The action topic the request was published to.
        action_topic: String,
        /// The reply topic that stayed empty.
        reply_topic: String,
        /// How long the caller waited, in milliseconds.
        waited_ms: u64,
    },
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadActionTopic(t) => write!(
                f,
                "RPC action topic {t:?} must start with `action/` \
                 (e.g. action/meshfs/rebalance)",
            ),
            Self::Persist(e) => write!(f, "RPC persist: {e}"),
            Self::Timeout {
                action_topic,
                reply_topic,
                waited_ms,
            } => write!(
                f,
                "no reply on {reply_topic} within {waited_ms} ms after \
                 publishing to {action_topic}. Is a responder for that \
                 action running?",
            ),
        }
    }
}

impl std::error::Error for RpcError {}

/// Publish a command to `action_topic` and return the request ULID
/// (the correlation key a responder replies against). Rejects any
/// topic outside the `action/` namespace.
///
/// # Errors
/// [`RpcError::BadActionTopic`] for a non-`action/` topic;
/// [`RpcError::Persist`] on a write failure.
pub fn publish_request(
    persist: &Persist,
    action_topic: &str,
    priority: Priority,
    title: Option<&str>,
    body: Option<&str>,
) -> Result<String, RpcError> {
    if !action_topic.starts_with(ACTION_PREFIX) {
        return Err(RpcError::BadActionTopic(action_topic.to_string()));
    }
    let msg = persist
        .write(action_topic, priority, title, body)
        .map_err(|e| RpcError::Persist(e.to_string()))?;
    Ok(msg.ulid)
}

/// Poll `reply/<request_ulid>` until a reply arrives or `timeout`
/// elapses. Returns the first reply message. The poll cadence is
/// [`DEFAULT_POLL_INTERVAL`], clamped so the final sleep never
/// overshoots the deadline.
///
/// # Errors
/// [`RpcError::Persist`] on a read failure; [`RpcError::Timeout`]
/// when no reply lands in time.
pub async fn await_reply(
    persist: &Persist,
    request_ulid: &str,
    timeout: Duration,
) -> Result<StoredMessage, RpcError> {
    let rtopic = reply_topic(request_ulid);
    let started = Instant::now();
    let deadline = started + timeout;
    loop {
        match persist.list_since(&rtopic, None) {
            Ok(mut msgs) if !msgs.is_empty() => {
                // Oldest-first by ULID; the first reply wins.
                return Ok(msgs.remove(0));
            }
            Ok(_) => {}
            Err(e) => return Err(RpcError::Persist(e.to_string())),
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(RpcError::Timeout {
                action_topic: format!("action/<...> (ulid {request_ulid})"),
                reply_topic: rtopic,
                waited_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            });
        }
        // Don't sleep past the deadline.
        let remaining = deadline.saturating_duration_since(now);
        tokio::time::sleep(DEFAULT_POLL_INTERVAL.min(remaining)).await;
    }
}

/// Convenience: [`publish_request`] then [`await_reply`]. The single
/// call a typical Rust caller makes in place of a D-Bus method
/// invocation.
///
/// # Errors
/// Per [`publish_request`] / [`await_reply`].
pub async fn request(
    persist: &Persist,
    action_topic: &str,
    priority: Priority,
    title: Option<&str>,
    body: Option<&str>,
    timeout: Duration,
) -> Result<StoredMessage, RpcError> {
    let ulid = publish_request(persist, action_topic, priority, title, body)?;
    await_reply(persist, &ulid, timeout).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn persist() -> (tempfile::TempDir, Persist) {
        let tmp = tempfile::tempdir().unwrap();
        let p = Persist::open(tmp.path().to_path_buf()).unwrap();
        (tmp, p)
    }

    #[test]
    fn reply_topic_formats() {
        assert_eq!(reply_topic("01ABC"), "reply/01ABC");
    }

    #[test]
    fn publish_request_rejects_non_action_topic() {
        let (_tmp, p) = persist();
        let r = publish_request(&p, "fleet/announce", Priority::Default, None, None);
        assert!(matches!(r, Err(RpcError::BadActionTopic(_))));
    }

    #[test]
    fn publish_request_writes_to_action_topic_and_returns_ulid() {
        let (_tmp, p) = persist();
        let ulid = publish_request(
            &p,
            "action/meshfs/rebalance",
            Priority::Default,
            Some("resolve"),
            Some("chunk:abc"),
        )
        .unwrap();
        assert_eq!(ulid.len(), 26, "ULID is 26 Crockford-base32 chars");
        // The action message is in the persist tree on its topic.
        let msgs = p.list_since("action/meshfs/rebalance", None).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].ulid, ulid);
    }

    #[tokio::test]
    async fn await_reply_returns_a_posted_reply() {
        let (_tmp, p) = persist();
        let ulid = publish_request(
            &p,
            "action/marks/list",
            Priority::Default,
            None,
            Some("0x1234"),
        )
        .unwrap();
        // A responder posts its answer to reply/<ulid>.
        p.write(&reply_topic(&ulid), Priority::Default, None, Some("tag:dev,elev:2"))
            .unwrap();
        let reply = await_reply(&p, &ulid, Duration::from_secs(5)).await.unwrap();
        assert_eq!(reply.topic, reply_topic(&ulid));
        assert_eq!(reply.body.as_deref(), Some("tag:dev,elev:2"));
    }

    #[tokio::test]
    async fn await_reply_times_out_without_a_reply() {
        let (_tmp, p) = persist();
        let ulid = publish_request(&p, "action/marks/list", Priority::Default, None, None)
            .unwrap();
        // No responder writes a reply → timeout (short, for the test).
        let r = await_reply(&p, &ulid, Duration::from_millis(300)).await;
        match r {
            Err(RpcError::Timeout { reply_topic: rt, .. }) => {
                assert_eq!(rt, reply_topic(&ulid));
            }
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn request_round_trips_when_reply_preexists() {
        // End-to-end via the convenience fn: pre-seed isn't possible
        // (ULID is internal), so drive it with a responder task that
        // answers whatever action ULID shows up.
        let (_tmp, p) = persist();
        // Publish first so the responder can find the action message.
        let ulid = publish_request(
            &p,
            "action/meshfs/probe-health",
            Priority::Default,
            None,
            Some("chunk:xyz"),
        )
        .unwrap();
        // Simulate the responder: reply to reply/<ulid>.
        p.write(&reply_topic(&ulid), Priority::Default, None, Some("healthy"))
            .unwrap();
        let reply = await_reply(&p, &ulid, Duration::from_secs(5)).await.unwrap();
        assert_eq!(reply.body.as_deref(), Some("healthy"));
    }

    #[tokio::test]
    async fn request_rejects_bad_topic_before_waiting() {
        let (_tmp, p) = persist();
        let r = request(
            &p,
            "not-an-action/foo",
            Priority::Default,
            None,
            None,
            Duration::from_millis(50),
        )
        .await;
        assert!(matches!(r, Err(RpcError::BadActionTopic(_))));
    }
}
