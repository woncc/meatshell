//! One close decision for the tab ×, the tab menu, and middle-click.
//! Quitting the app and batch closes do not use this.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripHit<'a> {
    Blank,
    NewTab,
    Tab { id: &'a str, kind: &'a str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloseIntent {
    Ignore,
    Close(String),
    Ask(String),
}

/// Middle-click only closes a real session tab. The new-tab button and empty
/// strip do nothing, and the click does not select or open anything.
pub fn middle_click(hit: StripHit<'_>, confirm: bool) -> CloseIntent {
    match hit {
        StripHit::Blank | StripHit::NewTab => CloseIntent::Ignore,
        StripHit::Tab { kind, .. } if kind == "welcome" => CloseIntent::Ignore,
        StripHit::Tab { id, .. } => gated(id, confirm),
    }
}

/// × and the tab-menu Close item. Same gate as middle-click.
pub fn button_close(tab_id: &str, confirm: bool) -> CloseIntent {
    if tab_id.is_empty() || tab_id == "welcome" {
        CloseIntent::Ignore
    } else {
        gated(tab_id, confirm)
    }
}

fn gated(tab_id: &str, confirm: bool) -> CloseIntent {
    if confirm {
        CloseIntent::Ask(tab_id.to_string())
    } else {
        CloseIntent::Close(tab_id.to_string())
    }
}

/// Horizontal hit test. Slots are in strip order. A slot with `id == None` is
/// the "+" button. Coordinates outside every slot are blank.
pub fn hit_strip<'a>(slots: &'a [(Option<&'a str>, f32, f32)], x: f32) -> StripHit<'a> {
    for (id, start, width) in slots {
        if x >= *start && x < start + width {
            return match id {
                None => StripHit::NewTab,
                Some(id) => StripHit::Tab { id, kind: "terminal" },
            };
        }
    }
    StripHit::Blank
}

/// Per-window confirmation. Cancel drops the pending id and never releases a
/// close. A second request replaces the pending tab without closing the first.
#[derive(Debug, Default)]
pub struct CloseGate {
    pending: Option<String>,
    release: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseStep {
    Ignore,
    Ask,
    Proceed,
}

impl CloseGate {
    pub fn begin(&mut self, tab_id: &str, confirm: bool) -> CloseStep {
        if tab_id.is_empty() || tab_id == "welcome" {
            return CloseStep::Ignore;
        }
        if self.release && self.pending.as_deref() == Some(tab_id) {
            self.pending = None;
            self.release = false;
            return CloseStep::Proceed;
        }
        if confirm {
            self.pending = Some(tab_id.to_string());
            self.release = false;
            return CloseStep::Ask;
        }
        self.pending = None;
        self.release = false;
        CloseStep::Proceed
    }

    pub fn confirm(&mut self) -> Option<String> {
        let id = self.pending.clone()?;
        self.release = true;
        Some(id)
    }

    pub fn cancel(&mut self) {
        self.pending = None;
        self.release = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_click_hits_only_the_tab_under_the_pointer() {
        let slots = [(Some("ssh-a"), 0.0, 80.0), (Some("zsh"), 82.0, 80.0), (None, 164.0, 28.0)];
        assert_eq!(
            middle_click(hit_strip(&slots, 10.0), false),
            CloseIntent::Close("ssh-a".into())
        );
        assert_eq!(
            middle_click(hit_strip(&slots, 100.0), false),
            CloseIntent::Close("zsh".into())
        );
        assert_eq!(middle_click(hit_strip(&slots, 170.0), false), CloseIntent::Ignore);
        assert_eq!(middle_click(hit_strip(&slots, 200.0), false), CloseIntent::Ignore);
        assert_eq!(
            middle_click(StripHit::Tab { id: "welcome", kind: "welcome" }, false),
            CloseIntent::Ignore
        );
    }

    #[test]
    fn default_closes_immediately_and_confirm_only_asks() {
        assert_eq!(button_close("zsh", false), CloseIntent::Close("zsh".into()));
        assert_eq!(middle_click(StripHit::Tab { id: "zsh", kind: "terminal" }, true), CloseIntent::Ask("zsh".into()));
        assert_eq!(button_close("ssh-a", true), CloseIntent::Ask("ssh-a".into()));
    }

    #[test]
    fn cancel_releases_nothing_and_a_repeat_replaces_the_pending_tab() {
        let mut gate = CloseGate::default();
        assert_eq!(gate.begin("zsh", true), CloseStep::Ask);
        gate.cancel();
        assert!(gate.confirm().is_none());
        assert_eq!(gate.begin("ssh-a", true), CloseStep::Ask);
        assert_eq!(gate.begin("bg", true), CloseStep::Ask);
        assert_eq!(gate.confirm().as_deref(), Some("bg"));
        assert_eq!(gate.begin("bg", true), CloseStep::Proceed);
        assert!(gate.confirm().is_none());
    }

    #[test]
    fn disconnected_and_background_tabs_use_the_same_gate() {
        for id in ["active", "background", "disconnected"] {
            assert_eq!(button_close(id, false), CloseIntent::Close(id.into()));
            assert_eq!(button_close(id, true), CloseIntent::Ask(id.into()));
        }
    }

    #[test]
    fn each_window_has_its_own_pending_close() {
        let mut left = CloseGate::default();
        let mut right = CloseGate::default();
        assert_eq!(left.begin("zsh", true), CloseStep::Ask);
        assert_eq!(right.begin("ssh-a", false), CloseStep::Proceed);
        left.cancel();
        assert!(left.confirm().is_none());
        assert!(right.confirm().is_none());
    }
}
