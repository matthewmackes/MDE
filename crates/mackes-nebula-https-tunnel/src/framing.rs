//! NF-1.3 — 4-byte length-prefixed framing.
//!
//! Wire format on the rustls stream:
//!
//! ```text
//!   ┌──────────┬─────────────────────────────────────────┐
//!   │ len (u32 │ payload (len bytes; ≤ MAX_FRAME_BYTES)  │
//!   │ big      │ — one Nebula UDP frame                  │
//!   │ endian)  │                                         │
//!   └──────────┴─────────────────────────────────────────┘
//! ```
//!
//! `MAX_FRAME_BYTES` is locked at Nebula's default MTU (1408)
//! per the v2.5 fabric design — frames larger than that won't
//! come out of the Nebula UDP socket on the inside of the
//! tunnel, so accepting a larger length on the wire would only
//! widen the attack surface for memory-amplification flooding.
//!
//! Pure functions on `BytesMut` — no I/O, no async; the
//! listener / dialer drive the buffered reads from the rustls
//! stream and call `decode_frame` per chunk.

use bytes::{Buf, BufMut, Bytes, BytesMut};

/// 4-byte big-endian length prefix size.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Maximum payload size in one frame (Nebula's default MTU).
/// Frames advertising a larger length are rejected at decode
/// time with [`FrameError::OversizedFrame`].
pub const MAX_FRAME_BYTES: usize = 1408;

/// Errors `decode_frame` can surface to its caller. Recoverable
/// vs. fatal is encoded by variant — `ShortRead` means "feed me
/// more bytes," the rest mean "tear the connection down."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Decoded length header exceeds `MAX_FRAME_BYTES`. Fatal —
    /// the peer is sending non-conformant frames; the caller
    /// closes the stream.
    OversizedFrame {
        /// Length the peer advertised in the prefix.
        announced: u32,
    },
}

impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OversizedFrame { announced } => write!(
                f,
                "frame length {announced} exceeds MAX_FRAME_BYTES ({MAX_FRAME_BYTES})"
            ),
        }
    }
}

impl std::error::Error for FrameError {}

/// Encode one payload into `out` as a 4-byte big-endian length
/// prefix followed by the payload bytes. Pure; no allocation
/// beyond what `BytesMut::put*` already does for capacity growth.
///
/// # Panics
///
/// Panics if `payload.len()` exceeds `u32::MAX`. The Nebula MTU
/// (1408) is many orders of magnitude below that ceiling, so the
/// panic is unreachable in production; tests covering the limit
/// would have to be deliberately pathological.
pub fn encode_frame(payload: &[u8], out: &mut BytesMut) {
    let len = u32::try_from(payload.len()).expect("payload fits in u32");
    out.reserve(LENGTH_PREFIX_BYTES + payload.len());
    out.put_u32(len);
    out.put_slice(payload);
}

/// Attempt to decode one frame off the front of `buf`. Returns:
///
///   * `Ok(Some(payload))` — one full frame was lifted out;
///     `buf` advances past the prefix + payload.
///   * `Ok(None)` — buffer has fewer than `4 + announced_len`
///     bytes; caller should read more from the underlying
///     stream and retry. `buf` is untouched in this case (no
///     destructive partial advance).
///   * `Err(FrameError::OversizedFrame)` — the announced length
///     exceeds the locked MTU ceiling; caller closes the stream.
///
/// Zero-length frames (`announced == 0`) decode successfully and
/// yield an empty `Bytes` — the upstream framing layer treats
/// them as keepalives.
///
/// # Errors
///
/// Returns [`FrameError::OversizedFrame`] when the announced
/// length exceeds [`MAX_FRAME_BYTES`]. The check fires as soon
/// as the 4-byte header arrives, so a hostile peer can't make
/// us allocate a large body buffer before we reject it.
pub fn decode_frame(buf: &mut BytesMut) -> Result<Option<Bytes>, FrameError> {
    if buf.len() < LENGTH_PREFIX_BYTES {
        return Ok(None);
    }

    let mut header = [0u8; LENGTH_PREFIX_BYTES];
    header.copy_from_slice(&buf[..LENGTH_PREFIX_BYTES]);
    let announced = u32::from_be_bytes(header);

    let announced_usize = announced as usize;
    if announced_usize > MAX_FRAME_BYTES {
        return Err(FrameError::OversizedFrame { announced });
    }

    if buf.len() < LENGTH_PREFIX_BYTES + announced_usize {
        return Ok(None);
    }

    buf.advance(LENGTH_PREFIX_BYTES);
    let payload = buf.split_to(announced_usize).freeze();
    Ok(Some(payload))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::needless_continue
)]
mod tests {
    use super::*;

    #[test]
    fn locked_max_frame_bytes_matches_nebula_mtu() {
        assert_eq!(
            MAX_FRAME_BYTES, 1408,
            "v2.5 fabric lock — changing this is a wire-protocol change"
        );
    }

    #[test]
    fn encode_then_decode_round_trip() {
        let payload = b"hello nebula";
        let mut out = BytesMut::new();
        encode_frame(payload, &mut out);

        let decoded = decode_frame(&mut out).expect("decode ok").expect("frame");
        assert_eq!(&decoded[..], payload);
        assert!(out.is_empty());
    }

    #[test]
    fn decode_short_read_returns_none_no_advance() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0u8, 0]);
        let before = buf.clone();
        assert_eq!(decode_frame(&mut buf).expect("decode ok"), None);
        assert_eq!(buf, before, "short read must not consume bytes");
    }

    #[test]
    fn decode_short_read_after_header_returns_none_no_advance() {
        // Header announces 10 bytes, only 4 follow → still partial.
        let mut buf = BytesMut::new();
        encode_frame(b"0123456789", &mut buf);
        // Truncate so only the header + 4 of the 10 payload bytes survive.
        let mut truncated = BytesMut::new();
        truncated.extend_from_slice(&buf[..LENGTH_PREFIX_BYTES + 4]);
        let before = truncated.clone();
        assert_eq!(decode_frame(&mut truncated).expect("decode ok"), None);
        assert_eq!(truncated, before, "partial body must not advance");
    }

    #[test]
    fn decode_oversized_frame_is_fatal() {
        let mut buf = BytesMut::new();
        let bogus_len = (MAX_FRAME_BYTES as u32) + 1;
        buf.put_u32(bogus_len);
        match decode_frame(&mut buf) {
            Err(FrameError::OversizedFrame { announced }) => {
                assert_eq!(announced, bogus_len);
            }
            other => panic!("expected OversizedFrame, got {other:?}"),
        }
    }

    #[test]
    fn decode_oversized_frame_caught_before_buffer_growth() {
        // Even if only the header is on the wire, the oversize
        // check fires — the peer doesn't get to make us allocate
        // a multi-MB body buffer just to find out it was lying.
        let mut buf = BytesMut::new();
        buf.put_u32(u32::MAX);
        assert!(matches!(
            decode_frame(&mut buf),
            Err(FrameError::OversizedFrame { announced: u32::MAX }),
        ));
    }

    #[test]
    fn decode_zero_length_frame_succeeds() {
        let mut buf = BytesMut::new();
        encode_frame(&[], &mut buf);
        let decoded = decode_frame(&mut buf).expect("decode ok").expect("frame");
        assert!(decoded.is_empty());
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_max_size_frame_is_accepted() {
        let payload = vec![0xAB; MAX_FRAME_BYTES];
        let mut buf = BytesMut::new();
        encode_frame(&payload, &mut buf);
        let decoded = decode_frame(&mut buf).expect("decode ok").expect("frame");
        assert_eq!(decoded.len(), MAX_FRAME_BYTES);
        assert!(decoded.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn decode_multi_frame_buffer_yields_each_in_order() {
        let mut buf = BytesMut::new();
        encode_frame(b"first", &mut buf);
        encode_frame(b"second", &mut buf);
        encode_frame(b"third", &mut buf);

        let a = decode_frame(&mut buf).expect("decode ok").expect("frame 1");
        let b = decode_frame(&mut buf).expect("decode ok").expect("frame 2");
        let c = decode_frame(&mut buf).expect("decode ok").expect("frame 3");
        assert_eq!(&a[..], b"first");
        assert_eq!(&b[..], b"second");
        assert_eq!(&c[..], b"third");
        assert_eq!(decode_frame(&mut buf).expect("decode ok"), None);
    }

    #[test]
    fn decode_partial_frame_across_multiple_reads() {
        // Simulate a stream that delivers a 200-byte frame in
        // 30-byte chunks: each decode call returns None until the
        // last byte arrives, then yields the full payload.
        let payload: Vec<u8> = (0..200u8).cycle().take(200).collect();
        let mut encoded = BytesMut::new();
        encode_frame(&payload, &mut encoded);

        let mut wire = BytesMut::new();
        let mut yielded = None;
        for chunk in encoded.chunks(30) {
            wire.extend_from_slice(chunk);
            if let Some(frame) = decode_frame(&mut wire).expect("decode ok") {
                yielded = Some(frame);
                break;
            }
        }
        let frame = yielded.expect("frame eventually completes");
        assert_eq!(&frame[..], &payload[..]);
        assert!(wire.is_empty());
    }

    #[test]
    fn decode_then_partial_next_holds_partial() {
        // After yielding one complete frame, leftover bytes that
        // form a partial second header stay in the buffer until
        // more arrive.
        let mut buf = BytesMut::new();
        encode_frame(b"complete", &mut buf);
        // Inject 2 bytes of a fake next-header prefix.
        buf.put_u8(0x00);
        buf.put_u8(0x05);

        let first = decode_frame(&mut buf).expect("decode ok").expect("frame 1");
        assert_eq!(&first[..], b"complete");
        // Partial header remains, no second frame yet.
        assert_eq!(decode_frame(&mut buf).expect("decode ok"), None);
        assert_eq!(buf.len(), 2);
    }
}
