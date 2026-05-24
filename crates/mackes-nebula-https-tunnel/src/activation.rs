//! NF-1.4 — Nebula-over-HTTPS activation state machine.
//!
//! Ported from the retired `mackesd::https_fallback`:
//!
//!   * Activates after **3 consecutive failed direct-UDP +
//!     lighthouse-relay probe pairs** within a 30 s window (one
//!     "failure cycle" = a direct-UDP probe failing AND its
//!     lighthouse-relay counterpart failing in the same window).
//!     Two failure cycles = wait; three = activate.
//!   * Once activated, stays activated until a fresh direct-UDP
//!     OR lighthouse-relay probe succeeds, at which point the
//!     router reverts to the upstream path.
//!
//! The wire-protocol side of this lives in [`crate::listen`] /
//! [`crate::dial`]; this module ships only the **policy** — the
//! failure-window detector + the activation state machine + the
//! pure-fn transition rules. Pure / data-only — testable in
//! microseconds.

use std::time::{Duration, Instant};

/// Observed outcome of one probe pair (direct-UDP +
/// lighthouse-relay) in a single observation window. The
/// connectivity worker emits one of these per probe cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbePairOutcome {
    /// At least one of (direct-UDP, lighthouse-relay) succeeded —
    /// the peer is reachable via a UDP path.
    AnyUdpSucceeded,
    /// Both direct-UDP and lighthouse-relay failed in the same
    /// window — the UDP-only path is wholly down.
    BothUdpFailed,
}

impl ProbePairOutcome {
    /// Stable string identifier for log lines + audit entries.
    /// Mirrors the snake-case `as_str` pattern used elsewhere in
    /// the workspace.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AnyUdpSucceeded => "any_udp_succeeded",
            Self::BothUdpFailed => "both_udp_failed",
        }
    }
}

impl core::fmt::Display for ProbePairOutcome {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Locked failure threshold. Three consecutive `BothUdpFailed`
/// outcomes within `FAILURE_WINDOW` = activate the HTTPS path.
pub const FAILURE_THRESHOLD: u32 = 3;

/// Locked observation window. Failures older than this fall off
/// the head of the sliding window — three failures across a
/// 30-second span trips activation; three failures across a
/// quiet hour does NOT.
pub const FAILURE_WINDOW: Duration = Duration::from_secs(30);

/// Sliding-window counter that tracks consecutive UDP-only
/// failures. Resets to 0 on any `AnyUdpSucceeded` observation
/// OR when the oldest failure ages past `FAILURE_WINDOW`.
///
/// The window is "consecutive failures within a 30 s span" —
/// the v2.5 lock from `docs/design/v2.5-nebula-fabric.md` Q4.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FailureWindow {
    /// Timestamps of failures still inside the window. Length
    /// caps at `FAILURE_THRESHOLD` — once we trip the threshold
    /// the caller resets us anyway.
    failures: Vec<Instant>,
}

impl FailureWindow {
    /// Construct a fresh window with no failures yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one probe-pair outcome at `now`. Returns the new
    /// failure count (after window-aging + reset semantics).
    ///
    /// Aging rule: every failure older than `now - FAILURE_WINDOW`
    /// is discarded before the new outcome is applied.
    pub fn observe_at(&mut self, outcome: ProbePairOutcome, now: Instant) -> u32 {
        let cutoff = now.checked_sub(FAILURE_WINDOW);
        if let Some(cutoff) = cutoff {
            self.failures.retain(|t| *t >= cutoff);
        }
        match outcome {
            ProbePairOutcome::BothUdpFailed => self.failures.push(now),
            ProbePairOutcome::AnyUdpSucceeded => self.failures.clear(),
        }
        self.consecutive_failures()
    }

    /// Convenience wrapper: observe at `Instant::now()`.
    pub fn observe(&mut self, outcome: ProbePairOutcome) -> u32 {
        self.observe_at(outcome, Instant::now())
    }

    /// Current consecutive failure count (after the most-recent
    /// `observe*` call). Does NOT age the window — call
    /// `observe_at` first if a freshly-aged read matters.
    #[must_use]
    pub fn consecutive_failures(&self) -> u32 {
        u32::try_from(self.failures.len()).unwrap_or(u32::MAX)
    }

    /// `true` when the failure count has reached `FAILURE_THRESHOLD`
    /// — caller should activate the HTTPS path.
    #[must_use]
    pub fn threshold_met(&self) -> bool {
        self.consecutive_failures() >= FAILURE_THRESHOLD
    }
}

/// HTTPS-tunnel activation state machine. The connectivity
/// worker drives transitions; the routing layer reads
/// `is_active()` to decide whether to spray packets over the
/// HTTPS path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    /// Default state. Direct-UDP / lighthouse-relay paths are
    /// healthy.
    #[default]
    Inactive,
    /// Failure threshold met; TLS handshake in flight. The panel
    /// surfaces a brief "connecting via HTTPS..." toast.
    Activating,
    /// Tunnel up + carrying Nebula frames. Routing layer sprays
    /// packets here.
    Active,
    /// Tunnel was up but the TLS handshake or the underlying TCP
    /// connection failed; reverting to the failure-window state.
    /// From Failing we go back to Inactive when a fresh UDP probe
    /// succeeds, OR back to Activating after one more threshold
    /// cycle.
    Failing,
}

impl State {
    /// `true` when the routing layer should send packets over
    /// the HTTPS tunnel. Active is the only state where traffic
    /// flows through the fallback; Activating means we're still
    /// in TLS handshake.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }

    /// `true` when the UI should surface the "connecting via
    /// HTTPS..." toast.
    #[must_use]
    pub const fn is_activating(self) -> bool {
        matches!(self, Self::Activating)
    }

    /// Stable string identifier for log lines + audit entries.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Activating => "activating",
            Self::Active => "active",
            Self::Failing => "failing",
        }
    }

    /// Parse from the stable string identifier; the inverse of
    /// [`State::as_str`]. Returns `None` for unrecognized inputs.
    ///
    /// Deliberately named `parse` rather than `from_str` so it
    /// doesn't shadow the standard [`std::str::FromStr`] trait
    /// signature (which would force a `FromStr::Err` type the
    /// pure-fn matrix doesn't need).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "inactive" => Some(Self::Inactive),
            "activating" => Some(Self::Activating),
            "active" => Some(Self::Active),
            "failing" => Some(Self::Failing),
            _ => None,
        }
    }
}

impl core::fmt::Display for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Inputs the pure-fn transition table accepts. The connectivity
/// worker calls [`transition`] with the current state + one of
/// these per tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionInput {
    /// One probe-pair outcome (direct-UDP + lighthouse-relay).
    Probe(ProbePairOutcome),
    /// TLS handshake completed successfully.
    HandshakeOk,
    /// TLS handshake failed.
    HandshakeFailed,
    /// Active tunnel's TCP connection broke.
    TunnelLost,
}

/// Apply one input to the (state, window) pair. Returns the new
/// state; the window is mutated in place.
///
/// Rules:
///
///   * `Inactive` + `Probe(BothUdpFailed)` ×3 → `Activating`.
///   * `Activating` + `HandshakeOk` → `Active`.
///   * `Activating` + `HandshakeFailed` → `Failing`.
///   * `Active` + `Probe(AnyUdpSucceeded)` → `Inactive` (revert).
///   * `Active` + `TunnelLost` → `Failing`.
///   * `Failing` + `Probe(AnyUdpSucceeded)` → `Inactive` (revert).
///   * `Failing` + `Probe(BothUdpFailed)` ×3 → `Activating` (retry).
#[must_use]
pub fn transition(state: State, window: &mut FailureWindow, input: TransitionInput) -> State {
    transition_at(state, window, input, Instant::now())
}

/// Same as [`transition`] but takes an explicit `now` — used by
/// the unit tests to drive the sliding window deterministically.
#[must_use]
pub fn transition_at(
    state: State,
    window: &mut FailureWindow,
    input: TransitionInput,
    now: Instant,
) -> State {
    match (state, input) {
        // From Inactive — tally failures, activate on threshold.
        (State::Inactive, TransitionInput::Probe(outcome)) => {
            window.observe_at(outcome, now);
            if window.threshold_met() {
                // Reset window so a re-entry into Inactive starts
                // clean (the next failure cycle counts from 0).
                *window = FailureWindow::new();
                State::Activating
            } else {
                State::Inactive
            }
        }
        // Handshake outcomes while Inactive are no-ops (shouldn't
        // happen in normal flow, but no harm if they do).
        (State::Inactive, _) => State::Inactive,

        // From Activating — wait for handshake outcome; ignore
        // probe outcomes (we'll re-tally once we're back in
        // Inactive or Failing).
        (State::Activating, TransitionInput::HandshakeOk) => State::Active,
        // HandshakeFailed (Activating) and TunnelLost (Active)
        // both flip into Failing — collapsed into one arm to
        // satisfy clippy::match_same_arms; the semantics are
        // unchanged from the source state machine.
        (State::Activating, TransitionInput::HandshakeFailed)
        | (State::Active, TransitionInput::TunnelLost) => State::Failing,
        (State::Activating, _) => State::Activating,

        // From Active — revert to Inactive on UDP recovery; flip
        // to Failing on tunnel loss (handled above); ignore
        // BothUdpFailed (we're already routing around it).
        //
        // The "recover to Inactive on AnyUdpSucceeded" body is
        // identical for Active and Failing; collapsed into one
        // arm — both reset the window and snap back to Inactive.
        (
            State::Active | State::Failing,
            TransitionInput::Probe(ProbePairOutcome::AnyUdpSucceeded),
        ) => {
            *window = FailureWindow::new();
            State::Inactive
        }
        (State::Active, _) => State::Active,

        // From Failing — recovery returns us to Inactive (handled
        // above); re-meeting the threshold retries Activating;
        // other inputs hold.
        (State::Failing, TransitionInput::Probe(ProbePairOutcome::BothUdpFailed)) => {
            window.observe_at(ProbePairOutcome::BothUdpFailed, now);
            if window.threshold_met() {
                *window = FailureWindow::new();
                State::Activating
            } else {
                State::Failing
            }
        }
        (State::Failing, _) => State::Failing,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn fail_at(n: u32, fw: &mut FailureWindow, now: Instant) -> u32 {
        let mut last = 0;
        for _ in 0..n {
            last = fw.observe_at(ProbePairOutcome::BothUdpFailed, now);
        }
        last
    }

    // --- FailureWindow -----------------------------------------------

    #[test]
    fn fresh_window_has_zero_failures() {
        let fw = FailureWindow::new();
        assert_eq!(fw.consecutive_failures(), 0);
        assert!(!fw.threshold_met());
    }

    #[test]
    fn observing_failures_accumulates() {
        let mut fw = FailureWindow::new();
        let t0 = Instant::now();
        assert_eq!(fw.observe_at(ProbePairOutcome::BothUdpFailed, t0), 1);
        assert_eq!(fw.observe_at(ProbePairOutcome::BothUdpFailed, t0), 2);
        assert_eq!(fw.observe_at(ProbePairOutcome::BothUdpFailed, t0), 3);
    }

    #[test]
    fn any_udp_success_resets_window() {
        let mut fw = FailureWindow::new();
        let t0 = Instant::now();
        fail_at(2, &mut fw, t0);
        assert_eq!(fw.consecutive_failures(), 2);
        fw.observe_at(ProbePairOutcome::AnyUdpSucceeded, t0);
        assert_eq!(fw.consecutive_failures(), 0);
        assert!(!fw.threshold_met());
    }

    #[test]
    fn threshold_met_at_three_consecutive_failures() {
        let mut fw = FailureWindow::new();
        let t0 = Instant::now();
        fail_at(2, &mut fw, t0);
        assert!(!fw.threshold_met());
        fail_at(1, &mut fw, t0);
        assert!(fw.threshold_met());
    }

    #[test]
    fn failures_older_than_window_age_off() {
        let mut fw = FailureWindow::new();
        let t0 = Instant::now();
        fail_at(2, &mut fw, t0);
        // 60 s later — well past the 30 s window. Old failures
        // age off when the next observe lands.
        let later = t0 + Duration::from_secs(60);
        assert_eq!(fw.observe_at(ProbePairOutcome::BothUdpFailed, later), 1);
        assert!(!fw.threshold_met());
    }

    // --- State ------------------------------------------------------

    #[test]
    fn default_state_is_inactive() {
        let s = State::default();
        assert_eq!(s, State::Inactive);
        assert!(!s.is_active());
        assert!(!s.is_activating());
    }

    #[test]
    fn is_active_only_for_active() {
        assert!(!State::Inactive.is_active());
        assert!(!State::Activating.is_active());
        assert!(State::Active.is_active());
        assert!(!State::Failing.is_active());
    }

    #[test]
    fn is_activating_only_for_activating() {
        assert!(!State::Inactive.is_activating());
        assert!(State::Activating.is_activating());
        assert!(!State::Active.is_activating());
        assert!(!State::Failing.is_activating());
    }

    #[test]
    fn as_str_round_trips_through_parse() {
        for s in [State::Inactive, State::Activating, State::Active, State::Failing] {
            assert_eq!(State::parse(s.as_str()), Some(s));
        }
        assert_eq!(State::parse("garbage"), None);
    }

    #[test]
    fn probe_outcome_as_str_round_trips() {
        assert_eq!(ProbePairOutcome::AnyUdpSucceeded.as_str(), "any_udp_succeeded");
        assert_eq!(ProbePairOutcome::BothUdpFailed.as_str(), "both_udp_failed");
    }

    // --- transition table -------------------------------------------

    #[test]
    fn inactive_to_activating_after_three_failures() {
        let mut fw = FailureWindow::new();
        let mut state = State::Inactive;
        let now = Instant::now();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(state, State::Inactive);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(state, State::Inactive);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(state, State::Activating);
        // Window is reset on activation so the next entry starts clean.
        assert_eq!(fw.consecutive_failures(), 0);
    }

    #[test]
    fn inactive_recovery_resets_window() {
        let mut fw = FailureWindow::new();
        let mut state = State::Inactive;
        let now = Instant::now();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        let good = TransitionInput::Probe(ProbePairOutcome::AnyUdpSucceeded);
        state = transition_at(state, &mut fw, bad, now);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(fw.consecutive_failures(), 2);
        state = transition_at(state, &mut fw, good, now);
        assert_eq!(state, State::Inactive);
        assert_eq!(fw.consecutive_failures(), 0);
    }

    #[test]
    fn activating_to_active_on_handshake_ok() {
        let mut fw = FailureWindow::new();
        let state = transition(State::Activating, &mut fw, TransitionInput::HandshakeOk);
        assert_eq!(state, State::Active);
    }

    #[test]
    fn activating_to_failing_on_handshake_failed() {
        let mut fw = FailureWindow::new();
        let state = transition(
            State::Activating,
            &mut fw,
            TransitionInput::HandshakeFailed,
        );
        assert_eq!(state, State::Failing);
    }

    #[test]
    fn activating_ignores_probe_inputs() {
        let mut fw = FailureWindow::new();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        let state = transition(State::Activating, &mut fw, bad);
        assert_eq!(state, State::Activating);
    }

    #[test]
    fn active_reverts_to_inactive_when_udp_recovers() {
        let mut fw = FailureWindow::new();
        let good = TransitionInput::Probe(ProbePairOutcome::AnyUdpSucceeded);
        let state = transition(State::Active, &mut fw, good);
        assert_eq!(state, State::Inactive);
    }

    #[test]
    fn active_flips_to_failing_on_tunnel_lost() {
        let mut fw = FailureWindow::new();
        let state = transition(State::Active, &mut fw, TransitionInput::TunnelLost);
        assert_eq!(state, State::Failing);
    }

    #[test]
    fn active_holds_on_both_udp_failed() {
        let mut fw = FailureWindow::new();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        let state = transition(State::Active, &mut fw, bad);
        assert_eq!(state, State::Active);
    }

    #[test]
    fn failing_recovers_to_inactive_on_udp_success() {
        let mut fw = FailureWindow::new();
        let good = TransitionInput::Probe(ProbePairOutcome::AnyUdpSucceeded);
        let state = transition(State::Failing, &mut fw, good);
        assert_eq!(state, State::Inactive);
    }

    #[test]
    fn failing_retries_activating_after_three_more_failures() {
        let mut fw = FailureWindow::new();
        let mut state = State::Failing;
        let now = Instant::now();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(state, State::Failing);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(state, State::Failing);
        state = transition_at(state, &mut fw, bad, now);
        assert_eq!(state, State::Activating);
    }

    #[test]
    fn failing_ignores_handshake_inputs() {
        let mut fw = FailureWindow::new();
        let s = transition(State::Failing, &mut fw, TransitionInput::HandshakeOk);
        assert_eq!(s, State::Failing);
        let s = transition(State::Failing, &mut fw, TransitionInput::TunnelLost);
        assert_eq!(s, State::Failing);
    }

    #[test]
    fn locked_failure_threshold_is_three() {
        assert_eq!(
            FAILURE_THRESHOLD, 3,
            "v2.5 Q4 lock — changing this is a wire-protocol change"
        );
    }

    #[test]
    fn locked_failure_window_is_thirty_seconds() {
        assert_eq!(
            FAILURE_WINDOW,
            Duration::from_secs(30),
            "v2.5 Q4 lock — changing this is a wire-protocol change"
        );
    }

    #[test]
    fn end_to_end_walk_through_full_lifecycle() {
        let mut fw = FailureWindow::new();
        let mut state = State::Inactive;
        let now = Instant::now();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        let good = TransitionInput::Probe(ProbePairOutcome::AnyUdpSucceeded);

        for _ in 0..3 {
            state = transition_at(state, &mut fw, bad, now);
        }
        assert_eq!(state, State::Activating);

        state = transition_at(state, &mut fw, TransitionInput::HandshakeOk, now);
        assert_eq!(state, State::Active);
        assert!(state.is_active());

        state = transition_at(state, &mut fw, good, now);
        assert_eq!(state, State::Inactive);
        assert!(!state.is_active());
    }

    #[test]
    fn end_to_end_handshake_failure_recovery_path() {
        let mut fw = FailureWindow::new();
        let mut state = State::Inactive;
        let now = Instant::now();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);

        for _ in 0..3 {
            state = transition_at(state, &mut fw, bad, now);
        }
        // Handshake fails on first attempt → Failing.
        state = transition_at(state, &mut fw, TransitionInput::HandshakeFailed, now);
        assert_eq!(state, State::Failing);
        // Three more failures → retry.
        for _ in 0..3 {
            state = transition_at(state, &mut fw, bad, now);
        }
        assert_eq!(state, State::Activating);
        // Handshake succeeds this time.
        state = transition_at(state, &mut fw, TransitionInput::HandshakeOk, now);
        assert_eq!(state, State::Active);
    }

    #[test]
    fn slow_failures_outside_window_never_activate() {
        // Three failures spaced 20 s apart: first and third fall
        // OUTSIDE the 30 s window relative to each other, so the
        // sliding count never reaches the threshold.
        let mut fw = FailureWindow::new();
        let mut state = State::Inactive;
        let t0 = Instant::now();
        let bad = TransitionInput::Probe(ProbePairOutcome::BothUdpFailed);
        for i in 0..5 {
            // 0 s, 20 s, 40 s, 60 s, 80 s — the previous failure
            // ages off before the next lands every time.
            let now = t0 + Duration::from_secs(20 * i);
            state = transition_at(state, &mut fw, bad, now);
        }
        assert_eq!(state, State::Inactive);
        // At each step the window holds at most 2 failures.
        assert!(fw.consecutive_failures() <= 2);
    }
}
