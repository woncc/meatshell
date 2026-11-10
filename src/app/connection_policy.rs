//! Session-list click policy. The default (`prefer_existing == false`) always
//! opens a new connection. When the user opts in, a click focuses a matching
//! tab in this window, and a double-click may open another one.

use std::time::{Duration, Instant};

use crate::config::SessionKind;

/// How long a second press must arrive to count as one double-click.
pub const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(280);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReusePrefs {
    pub prefer_existing: bool,
    /// Ignored unless `prefer_existing` is on.
    pub allow_double_open: bool,
    /// Ignored unless both switches above are on.
    pub confirm_extra: bool,
}

impl Default for ReusePrefs {
    fn default() -> Self {
        Self {
            prefer_existing: false,
            allow_double_open: true,
            confirm_extra: true,
        }
    }
}

/// Live state of one tab in the current window. `0` connecting, `1` connected,
/// `2` disconnected — the same codes stored on `TabStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    Connected,
    Disconnected,
}

impl Phase {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Connected,
            2 => Self::Disconnected,
            _ => Self::Connecting,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Connected => 0,
            Self::Connecting => 1,
            Self::Disconnected => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenTab {
    pub tab_id: String,
    pub session_id: String,
    pub kind: SessionKind,
    pub phase: Phase,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    Single,
    Double,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenDecision {
    OpenNew,
    Focus(String),
    ConfirmOpen,
}

/// What a finished click or double-click should do.
///
/// Matching uses the saved session id and its protocol, never the display
/// name. `tabs` must already be limited to the current window. RDP has no
/// in-app tab, so it always opens the system client. Several matches prefer a
/// connected tab, then one still connecting, then a disconnected tab that can
/// be reconnected with Enter. The active tab wins ties; otherwise the later
/// tab in `tabs` wins.
pub fn decide(
    prefs: ReusePrefs,
    gesture: Gesture,
    session_id: &str,
    kind: SessionKind,
    tabs: &[OpenTab],
) -> OpenDecision {
    if kind == SessionKind::Rdp || !prefs.prefer_existing {
        return OpenDecision::OpenNew;
    }
    let best = best_match(session_id, kind, tabs);
    match gesture {
        Gesture::Single => match best {
            Some(tab) => OpenDecision::Focus(tab.tab_id.clone()),
            None => OpenDecision::OpenNew,
        },
        Gesture::Double => {
            if !prefs.allow_double_open {
                return match best {
                    Some(tab) => OpenDecision::Focus(tab.tab_id.clone()),
                    None => OpenDecision::OpenNew,
                };
            }
            if best.is_some() && prefs.confirm_extra {
                OpenDecision::ConfirmOpen
            } else {
                OpenDecision::OpenNew
            }
        }
    }
}

fn best_match<'a>(
    session_id: &str,
    kind: SessionKind,
    tabs: &'a [OpenTab],
) -> Option<&'a OpenTab> {
    let mut best: Option<&OpenTab> = None;
    for tab in tabs {
        if tab.session_id != session_id || tab.kind != kind {
            continue;
        }
        best = Some(match best {
            None => tab,
            Some(current) if replaces(current, tab) => tab,
            Some(current) => current,
        });
    }
    best
}

fn replaces(current: &OpenTab, next: &OpenTab) -> bool {
    let current_rank = current.phase.rank();
    let next_rank = next.phase.rank();
    if next_rank != current_rank {
        return next_rank < current_rank;
    }
    if next.active != current.active {
        return next.active;
    }
    true
}

/// Collapses two presses of the same session into one double-click so a
/// double-click cannot also run the single-click action.
#[derive(Debug, Default)]
pub struct ClickArbiter {
    pending_id: Option<String>,
    pending_at: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArbiterStep {
    /// Wait `DOUBLE_CLICK_WINDOW` before treating this as a single click.
    Arm { session_id: String },
    FireDouble { session_id: String },
}

impl ClickArbiter {
    pub fn click(&mut self, session_id: &str, now: Instant) -> ArbiterStep {
        if let (Some(id), Some(at)) = (&self.pending_id, self.pending_at) {
            if id == session_id && now.saturating_duration_since(at) <= DOUBLE_CLICK_WINDOW {
                self.clear();
                return ArbiterStep::FireDouble {
                    session_id: session_id.to_string(),
                };
            }
        }
        self.pending_id = Some(session_id.to_string());
        self.pending_at = Some(now);
        ArbiterStep::Arm {
            session_id: session_id.to_string(),
        }
    }

    /// `Some` when the armed single click has waited long enough.
    pub fn expire(&mut self, now: Instant) -> Option<String> {
        match (&self.pending_id, self.pending_at) {
            (Some(id), Some(at)) if now.saturating_duration_since(at) >= DOUBLE_CLICK_WINDOW => {
                let id = id.clone();
                self.clear();
                Some(id)
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.pending_id = None;
        self.pending_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs(prefer: bool, double_open: bool, confirm: bool) -> ReusePrefs {
        ReusePrefs {
            prefer_existing: prefer,
            allow_double_open: double_open,
            confirm_extra: confirm,
        }
    }

    fn tab(id: &str, session: &str, kind: SessionKind, phase: Phase, active: bool) -> OpenTab {
        OpenTab {
            tab_id: id.into(),
            session_id: session.into(),
            kind,
            phase,
            active,
        }
    }

    #[test]
    fn default_off_always_opens_even_when_a_tab_exists() {
        let tabs = [tab("t1", "s", SessionKind::Ssh, Phase::Connected, true)];
        let off = prefs(false, true, true);
        assert_eq!(
            decide(off, Gesture::Single, "s", SessionKind::Ssh, &tabs),
            OpenDecision::OpenNew
        );
        assert_eq!(
            decide(off, Gesture::Double, "s", SessionKind::Ssh, &tabs),
            OpenDecision::OpenNew
        );
    }

    #[test]
    fn single_click_focuses_same_id_and_protocol_in_this_window() {
        let tabs = [
            tab("other-name", "s", SessionKind::Telnet, Phase::Connected, false),
            tab("ssh", "s", SessionKind::Ssh, Phase::Disconnected, false),
        ];
        let on = prefs(true, true, true);
        assert_eq!(
            decide(on, Gesture::Single, "s", SessionKind::Ssh, &tabs),
            OpenDecision::Focus("ssh".into())
        );
        assert_eq!(
            decide(on, Gesture::Single, "missing", SessionKind::Ssh, &tabs),
            OpenDecision::OpenNew
        );
    }

    #[test]
    fn same_display_name_is_not_enough_to_reuse() {
        let tabs = [tab("t", "saved-a", SessionKind::Ssh, Phase::Connected, true)];
        let on = prefs(true, false, false);
        assert_eq!(
            decide(on, Gesture::Single, "saved-b", SessionKind::Ssh, &tabs),
            OpenDecision::OpenNew
        );
    }

    #[test]
    fn prefers_connected_then_connecting_then_disconnected() {
        let tabs = [
            tab("down", "s", SessionKind::Ssh, Phase::Disconnected, true),
            tab("wait", "s", SessionKind::Ssh, Phase::Connecting, false),
            tab("up", "s", SessionKind::Ssh, Phase::Connected, false),
        ];
        let on = prefs(true, false, false);
        assert_eq!(
            decide(on, Gesture::Single, "s", SessionKind::Ssh, &tabs),
            OpenDecision::Focus("up".into())
        );
        let tabs = [
            tab("down", "s", SessionKind::Ssh, Phase::Disconnected, true),
            tab("wait", "s", SessionKind::Ssh, Phase::Connecting, false),
        ];
        assert_eq!(
            decide(on, Gesture::Single, "s", SessionKind::Ssh, &tabs),
            OpenDecision::Focus("wait".into())
        );
    }

    #[test]
    fn active_tab_wins_a_tie_otherwise_the_later_tab() {
        let on = prefs(true, false, false);
        let tabs = [
            tab("a", "s", SessionKind::Ssh, Phase::Connected, false),
            tab("b", "s", SessionKind::Ssh, Phase::Connected, true),
            tab("c", "s", SessionKind::Ssh, Phase::Connected, false),
        ];
        assert_eq!(
            decide(on, Gesture::Single, "s", SessionKind::Ssh, &tabs),
            OpenDecision::Focus("b".into())
        );
        let tabs = [
            tab("a", "s", SessionKind::Ssh, Phase::Connected, false),
            tab("c", "s", SessionKind::Ssh, Phase::Connected, false),
        ];
        assert_eq!(
            decide(on, Gesture::Single, "s", SessionKind::Ssh, &tabs),
            OpenDecision::Focus("c".into())
        );
    }

    #[test]
    fn double_click_respects_allow_and_confirm() {
        let tabs = [tab("t", "s", SessionKind::Ssh, Phase::Connected, true)];
        assert_eq!(
            decide(prefs(true, false, true), Gesture::Double, "s", SessionKind::Ssh, &tabs),
            OpenDecision::Focus("t".into())
        );
        assert_eq!(
            decide(prefs(true, true, false), Gesture::Double, "s", SessionKind::Ssh, &tabs),
            OpenDecision::OpenNew
        );
        assert_eq!(
            decide(prefs(true, true, true), Gesture::Double, "s", SessionKind::Ssh, &tabs),
            OpenDecision::ConfirmOpen
        );
        assert_eq!(
            decide(prefs(true, true, true), Gesture::Double, "s", SessionKind::Ssh, &[]),
            OpenDecision::OpenNew
        );
    }

    #[test]
    fn rdp_always_opens_the_system_client() {
        let tabs = [tab("t", "s", SessionKind::Rdp, Phase::Connected, true)];
        assert_eq!(
            decide(prefs(true, true, true), Gesture::Single, "s", SessionKind::Rdp, &tabs),
            OpenDecision::OpenNew
        );
    }

    #[test]
    fn one_double_click_is_one_gesture() {
        let start = Instant::now();
        let mut arbiter = ClickArbiter::default();
        assert!(matches!(
            arbiter.click("s", start),
            ArbiterStep::Arm { .. }
        ));
        assert!(arbiter.expire(start + Duration::from_millis(100)).is_none());
        assert_eq!(
            arbiter.click("s", start + Duration::from_millis(120)),
            ArbiterStep::FireDouble { session_id: "s".into() }
        );
        assert!(arbiter.expire(start + DOUBLE_CLICK_WINDOW + Duration::from_millis(50)).is_none());
    }

    #[test]
    fn a_single_click_fires_only_after_the_window() {
        let start = Instant::now();
        let mut arbiter = ClickArbiter::default();
        arbiter.click("s", start);
        assert!(arbiter.expire(start + Duration::from_millis(279)).is_none());
        assert_eq!(
            arbiter.expire(start + DOUBLE_CLICK_WINDOW),
            Some("s".into())
        );
        assert!(arbiter.expire(start + DOUBLE_CLICK_WINDOW + Duration::from_millis(10)).is_none());
    }

    #[test]
    fn a_different_session_does_not_complete_the_previous_double_click() {
        let start = Instant::now();
        let mut arbiter = ClickArbiter::default();
        arbiter.click("a", start);
        assert!(matches!(
            arbiter.click("b", start + Duration::from_millis(40)),
            ArbiterStep::Arm { session_id } if session_id == "b"
        ));
        assert_eq!(
            arbiter.expire(start + DOUBLE_CLICK_WINDOW + Duration::from_millis(40)),
            Some("b".into())
        );
    }
}
