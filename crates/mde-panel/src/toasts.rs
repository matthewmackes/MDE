//! Phase E.20 — bottom-edge transient toast popups.
//!
//! Toasts are short-lived overlay messages — "copied!", "saved",
//! "linked to peer lab-01". They appear above the bottom-bar
//! panel surface for a fixed duration, then fade.
//!
//! 2026 design language:
//! - Centered on the bottom edge, 24px above the panel.
//! - Pill shape: 12px corner radius, 8px vertical / 16px horizontal
//!   padding, hairline border in `mackes_accent` at 22% alpha.
//! - 2 second visible duration, 220ms fade-in + 320ms fade-out.
//! - Stack vertically when multiple toasts queue, newest on top.
//! - Drop the longest-visible toast if the stack exceeds 3.

use std::time::{Duration, Instant};

/// Default visible duration (excluding fade in/out).
pub const DEFAULT_VISIBLE_MS: u64 = 2000;
/// Stack ceiling — drop the oldest when this is hit.
pub const STACK_LIMIT: usize = 3;

/// Severity styles the renderer can pick up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastKind {
    #[default]
    Info,
    Success,
    Warn,
    Error,
}

/// One toast in the stack.
#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub body: String,
    pub created_at: Instant,
    pub visible_for: Duration,
}

impl Toast {
    /// Create with the default duration.
    #[must_use]
    pub fn info<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Info,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    #[must_use]
    pub fn success<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Success,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    #[must_use]
    pub fn warn<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Warn,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    #[must_use]
    pub fn error<S: Into<String>>(body: S) -> Self {
        Self::with(
            ToastKind::Error,
            body,
            Duration::from_millis(DEFAULT_VISIBLE_MS),
        )
    }

    fn with<S: Into<String>>(kind: ToastKind, body: S, visible_for: Duration) -> Self {
        Self {
            kind,
            body: body.into(),
            created_at: Instant::now(),
            visible_for,
        }
    }

    /// True once the toast's visible window has elapsed.
    #[must_use]
    pub fn is_expired_at(&self, now: Instant) -> bool {
        now.duration_since(self.created_at) >= self.visible_for
    }
}

/// The toast stack — bounded queue with FIFO eviction.
#[derive(Debug, Clone, Default)]
pub struct ToastStack {
    inner: Vec<Toast>,
}

impl ToastStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new toast onto the stack. Evicts the oldest if the
    /// stack would exceed `STACK_LIMIT`.
    pub fn push(&mut self, toast: Toast) {
        self.inner.push(toast);
        if self.inner.len() > STACK_LIMIT {
            self.inner.remove(0);
        }
    }

    /// Remove expired toasts. Caller invokes this on each tick.
    pub fn retain_unexpired(&mut self, now: Instant) {
        self.inner.retain(|t| !t.is_expired_at(now));
    }

    /// Current visible count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True when no toasts are visible.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate from oldest (bottom of stack) to newest (top).
    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.inner.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_constructor_has_info_kind() {
        let t = Toast::info("hello");
        assert_eq!(t.kind, ToastKind::Info);
        assert_eq!(t.body, "hello");
    }

    #[test]
    fn variant_constructors_set_correct_kind() {
        assert_eq!(Toast::success("ok").kind, ToastKind::Success);
        assert_eq!(Toast::warn("uh").kind, ToastKind::Warn);
        assert_eq!(Toast::error("no").kind, ToastKind::Error);
    }

    #[test]
    fn is_expired_after_visible_window() {
        let mut t = Toast::info("body");
        t.visible_for = Duration::from_millis(100);
        let later = t.created_at + Duration::from_millis(150);
        assert!(t.is_expired_at(later));
    }

    #[test]
    fn is_not_expired_before_visible_window() {
        let mut t = Toast::info("body");
        t.visible_for = Duration::from_millis(2000);
        let earlier = t.created_at + Duration::from_millis(500);
        assert!(!t.is_expired_at(earlier));
    }

    #[test]
    fn stack_starts_empty() {
        let stack = ToastStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn stack_push_adds_a_toast() {
        let mut stack = ToastStack::new();
        stack.push(Toast::info("a"));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn stack_evicts_oldest_when_over_limit() {
        let mut stack = ToastStack::new();
        stack.push(Toast::info("oldest"));
        stack.push(Toast::info("middle"));
        stack.push(Toast::info("newest-1"));
        stack.push(Toast::info("newest-2")); // 4th — exceeds STACK_LIMIT=3
        assert_eq!(stack.len(), STACK_LIMIT);
        // The "oldest" was evicted.
        let bodies: Vec<&str> = stack.iter().map(|t| t.body.as_str()).collect();
        assert!(!bodies.contains(&"oldest"));
        assert_eq!(bodies, vec!["middle", "newest-1", "newest-2"]);
    }

    #[test]
    fn retain_unexpired_drops_expired_toasts() {
        let mut stack = ToastStack::new();
        let mut t = Toast::info("expired");
        t.visible_for = Duration::from_millis(10);
        stack.push(t);
        let later = Instant::now() + Duration::from_millis(500);
        stack.retain_unexpired(later);
        assert!(stack.is_empty());
    }

    #[test]
    fn default_visible_window_is_2000ms() {
        assert_eq!(DEFAULT_VISIBLE_MS, 2000);
    }

    #[test]
    fn stack_limit_is_3() {
        assert_eq!(STACK_LIMIT, 3);
    }
}
