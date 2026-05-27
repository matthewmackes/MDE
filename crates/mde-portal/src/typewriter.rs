//! Portal-14.a (v6.0, R4-Q22 / R4-Q24) — typewriter reveal primitive.
//!
//! Every breadcrumb segment in the Dock (BUS-2.2 fleet/announce
//! pills, Portal-57.c.basic urgent pulses, Portal-43 prev-workspace
//! cells, etc.) carries a `spawned_at` timestamp. The typewriter
//! primitive surfaces text char-by-char at 60 chars/sec from
//! `spawned_at` onward — a Linear-style polish that signals
//! "this just arrived" without being decorative.
//!
//! Pure functions only; no Iced widgets. Consumers wrap their
//! existing `text(...)` call with `typewriter_visible_text(...)`
//! at render time. The 33-ms typewriter subscription drives
//! Iced re-renders while any segment is mid-reveal so the
//! progress is smooth.
//!
//! ## Why 33 ms cadence at 60 chars/sec
//!
//! 60 chars/sec = ~16.7 ms/char. Driving the subscription at
//! 16 ms would burn CPU. 33 ms = 2 chars/tick at 60 chars/sec
//! which renders smoothly enough that the eye sees a continuous
//! reveal. The Iced render loop coalesces multiple Message
//! arrivals per frame, so the effective frame rate is bounded
//! by wgpu's vsync (16-17 ms typical), not by tick rate.
//!
//! ## Reverse typewriter (R4-Q24)
//!
//! Forward-reveal ships with Portal-14.a. Reverse-on-dismiss is
//! decorative — most breadcrumb segments auto-dismiss via TTL
//! without a visible exit. If the dismiss-fade design lock ends
//! up requiring it, Portal-14.a.reverse layers on top.

use chrono::{DateTime, Local};

/// Default char-reveal rate per the R4-Q22 design lock.
/// 60 chars/sec ≈ Linear's notification-row reveal pacing.
pub const DEFAULT_CHARS_PER_SEC: f64 = 60.0;

/// Typewriter subscription tick interval. 33 ms = ~30 Hz; 2
/// chars revealed per tick at 60 chars/sec. Fast enough for
/// smooth perception, slow enough to stay cheap.
pub const TICK_INTERVAL_MS: u64 = 33;

/// Return the prefix of `target` revealed at time `now` based
/// on the linear char-reveal rate `chars_per_sec` since
/// `spawned_at`. Returns the full `target` once enough time
/// has elapsed to reveal every char.
///
/// UTF-8-safe: progresses by char-boundary indices, not byte
/// offsets, so multi-byte characters (`é`, emoji, etc.) never
/// surface as half-byte garbage.
///
/// Negative elapsed time (e.g. clock-skew or future spawned_at)
/// returns the empty prefix.
#[must_use]
pub fn typewriter_visible_text<'a>(
    target: &'a str,
    spawned_at: DateTime<Local>,
    now: DateTime<Local>,
    chars_per_sec: f64,
) -> &'a str {
    let elapsed_ms = now.signed_duration_since(spawned_at).num_milliseconds();
    if elapsed_ms < 0 {
        return "";
    }
    let visible_chars = ((elapsed_ms as f64) * chars_per_sec / 1000.0).floor() as usize;
    // Walk char boundaries to find the byte-offset that ends the
    // `visible_chars`-th character. char_indices yields (byte_index,
    // char) pairs; the byte_index at position N is the START byte
    // of char N, so we want char_indices().nth(visible_chars) to
    // get the end-byte of char (visible_chars - 1).
    if visible_chars == 0 {
        return "";
    }
    let mut iter = target.char_indices();
    // Advance past `visible_chars - 1` chars, then the byte_index
    // of the next char (or end-of-string) is our split point.
    for _ in 0..visible_chars {
        if iter.next().is_none() {
            return target; // Past end → full reveal.
        }
    }
    match iter.next() {
        Some((byte_idx, _)) => &target[..byte_idx],
        None => target,
    }
}

/// `true` if the typewriter for `target` is still revealing at
/// `now`. Returns `false` once all chars are visible — consumers
/// can use this to deactivate the typewriter subscription when
/// no segment is mid-reveal.
#[must_use]
pub fn typewriter_still_revealing(
    target: &str,
    spawned_at: DateTime<Local>,
    now: DateTime<Local>,
    chars_per_sec: f64,
) -> bool {
    let total_chars = target.chars().count();
    if total_chars == 0 {
        return false;
    }
    let elapsed_ms = now.signed_duration_since(spawned_at).num_milliseconds();
    if elapsed_ms < 0 {
        return true; // Pre-start → still revealing.
    }
    let total_reveal_ms = (total_chars as f64 / chars_per_sec) * 1000.0;
    (elapsed_ms as f64) < total_reveal_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms_after: i64) -> DateTime<Local> {
        let base = Local::now();
        base + chrono::Duration::milliseconds(ms_after)
    }

    /// At t=0, nothing is revealed.
    #[test]
    fn at_zero_elapsed_nothing_revealed() {
        let now = at(0);
        let spawned = now;
        let s = typewriter_visible_text("hello world", spawned, now, 60.0);
        assert_eq!(s, "");
    }

    /// At 60 chars/sec, 250ms elapsed = 15 chars revealed.
    #[test]
    fn fifteen_chars_after_250ms() {
        let spawned = at(0);
        let now = at(250);
        let s = typewriter_visible_text("the quick brown fox jumps", spawned, now, 60.0);
        // 250ms * 60/1000 = 15.0 chars.
        assert_eq!(s, "the quick brown");
    }

    /// At 60 chars/sec, 500ms elapsed = 30 chars.
    #[test]
    fn thirty_chars_after_500ms() {
        let spawned = at(0);
        let now = at(500);
        let target = "the quick brown fox jumps over the lazy dog";
        let s = typewriter_visible_text(target, spawned, now, 60.0);
        assert_eq!(s, "the quick brown fox jumps over");
    }

    /// At 60 chars/sec, 2000ms elapsed = 120 chars — well past
    /// a 25-char target → full reveal.
    #[test]
    fn full_reveal_after_target_length_elapsed() {
        let spawned = at(0);
        let now = at(2000);
        let target = "hello world";
        let s = typewriter_visible_text(target, spawned, now, 60.0);
        assert_eq!(s, target);
    }

    /// Multi-byte UTF-8 characters never split mid-byte.
    #[test]
    fn utf8_multibyte_safe() {
        let spawned = at(0);
        let target = "héllo wörld";
        // 250ms * 60/1000 = 15 chars — full reveal of an 11-char string.
        let s = typewriter_visible_text(target, spawned, at(250), 60.0);
        assert_eq!(s, target);
        // 100ms = 6 chars → "héllo " (6 chars including the space).
        let s = typewriter_visible_text(target, spawned, at(100), 60.0);
        assert_eq!(s, "héllo ");
        // 33ms = 1.98 chars → floor = 1 char → "h".
        let s = typewriter_visible_text(target, spawned, at(33), 60.0);
        assert_eq!(s, "h");
    }

    /// Emoji + grapheme-cluster characters (4-byte UTF-8) work.
    #[test]
    fn emoji_chars_round_trip() {
        let spawned = at(0);
        let target = "✓ done 🎉";
        // After 50ms = 3 chars → "✓ d".
        let s = typewriter_visible_text(target, spawned, at(50), 60.0);
        assert_eq!(s, "✓ d");
        // After 200ms = 12 chars → full reveal of 8-char target.
        let s = typewriter_visible_text(target, spawned, at(200), 60.0);
        assert_eq!(s, target);
    }

    /// Empty target always returns empty regardless of elapsed.
    #[test]
    fn empty_target_returns_empty() {
        let spawned = at(0);
        let s = typewriter_visible_text("", spawned, at(1000), 60.0);
        assert_eq!(s, "");
    }

    /// Negative elapsed time (now < spawned_at) returns empty.
    #[test]
    fn negative_elapsed_returns_empty() {
        let now = at(0);
        let spawned = at(500);
        let s = typewriter_visible_text("hello", spawned, now, 60.0);
        assert_eq!(s, "");
    }

    /// `typewriter_still_revealing` returns true mid-reveal,
    /// false once full reveal completes.
    #[test]
    fn still_revealing_tracks_completion() {
        let spawned = at(0);
        // "hello world" is 11 chars; at 60 chars/sec full reveal
        // takes 11/60 * 1000 ≈ 183 ms.
        assert!(typewriter_still_revealing("hello world", spawned, at(0), 60.0));
        assert!(typewriter_still_revealing("hello world", spawned, at(150), 60.0));
        // 184 ms is past completion.
        assert!(!typewriter_still_revealing("hello world", spawned, at(184), 60.0));
        assert!(!typewriter_still_revealing("hello world", spawned, at(500), 60.0));
    }

    /// `typewriter_still_revealing` returns false for empty
    /// target.
    #[test]
    fn still_revealing_empty_target_is_complete() {
        let spawned = at(0);
        assert!(!typewriter_still_revealing("", spawned, at(0), 60.0));
    }

    /// Different chars-per-sec rates produce proportionally
    /// different progress.
    #[test]
    fn chars_per_sec_scales_reveal_rate() {
        let spawned = at(0);
        let target = "abcdefghij";
        // 30 chars/sec → 250ms = 7.5 → 7 chars.
        let s = typewriter_visible_text(target, spawned, at(250), 30.0);
        assert_eq!(s, "abcdefg");
        // 120 chars/sec → 250ms = 30 chars → full reveal.
        let s = typewriter_visible_text(target, spawned, at(250), 120.0);
        assert_eq!(s, target);
    }

    /// Default rate const matches the R4-Q22 design lock.
    #[test]
    fn default_rate_matches_design_lock() {
        assert!((DEFAULT_CHARS_PER_SEC - 60.0).abs() < f64::EPSILON);
    }
}
