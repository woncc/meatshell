//! Geometry and hit-testing of the main window's terminal and SFTP areas.

use super::*;

/// The active terminal tab's current SFTP directory ("" if unknown).
pub(super) fn active_sftp_path(win: &AppWindow, tab_id: &str) -> String {
    let model = win.get_terminals();
    if let Some(m) = model.as_any().downcast_ref::<VecModel<TerminalState>>() {
        for i in 0..m.row_count() {
            if let Some(row) = m.row_data(i) {
                if row.id.as_str() == tab_id {
                    return row.sftp_path.to_string();
                }
            }
        }
    }
    String::new()
}

// The raw macOS wheel fallback runs before the usual Slint hit testing. Keep
// modal-state routing explicit so it cannot target a terminal behind a dialog.
pub(super) fn macos_terminal_wheel_can_target_terminal(modal_open: bool) -> bool {
    !modal_open
}

pub(super) fn terminal_wheel_hit(
    win: &AppWindow,
    bufs: &TermBuffers,
    x: f32,
    y: f32,
) -> Option<TerminalWheelHit> {
    let (active, term, term_state) = active_terminal_panel_rects(win)?;
    let mut term_x = term.x;
    let mut term_y = term.y;
    let mut term_w = term.w;
    let mut term_h = term.h;

    // Zen mode removes the 24px status strip. The SFTP panel is hidden too,
    // so it must not keep eating wheel hits.
    if !win.get_zen_mode() {
        term_y += 24.0;
        term_h = (term_h - 24.0).max(0.0);
    }

    let sftp_dock = win.get_sftp_dock().to_string();
    let expanded = if sftp_dock == "left" || sftp_dock == "right" {
        term_state.sftp_panel_width
    } else {
        term_state.sftp_panel_height
    };
    let sftp_take = sftp_panel_take(
        win.get_sftp_enabled(),
        win.get_zen_mode(),
        term_state.sftp_collapsed,
        expanded,
    );
    shrink_edge(
        &mut term_x,
        &mut term_y,
        &mut term_w,
        &mut term_h,
        &sftp_dock,
        sftp_take,
    );

    // Leave the command bar to TextInput/history handling; wheel fallback is
    // for terminal output only. The bar is present in every mode unless the
    // toolbar toggle hid it.
    if !win.get_cmd_bar_hidden() {
        term_h = (term_h - 34.0).max(0.0);
    }
    if !contains_logical(
        LogicalRect {
            x: term_x,
            y: term_y,
            w: term_w,
            h: term_h,
        },
        x,
        y,
    ) {
        return None;
    }

    let h = term_buf(bufs, &active)?;
    let guard = h.lock().ok()?;
    let screen = guard.parser.screen();
    let (rows, cols) = screen.size();
    let cell_w = (term_w / cols.max(1) as f32).max(1.0);
    let cell_h = (term_h / rows.max(1) as f32).max(1.0);
    Some(TerminalWheelHit {
        tab_id: active,
        is_alt: screen.alternate_screen(),
        col: ((x - term_x) / cell_w).floor() as i32,
        row: ((y - term_y) / cell_h).floor() as i32,
    })
}

pub(super) fn shrink_edge(
    x: &mut f32,
    y: &mut f32,
    w: &mut f32,
    h: &mut f32,
    dock: &str,
    amount: f32,
) {
    let amount = amount.max(0.0);
    match dock {
        "left" => {
            *x += amount;
            *w = (*w - amount).max(0.0);
        }
        "right" => *w = (*w - amount).max(0.0),
        "top" => {
            *y += amount;
            *h = (*h - amount).max(0.0);
        }
        "bottom" => *h = (*h - amount).max(0.0),
        _ => {}
    }
}

pub(super) fn contains_logical(rect: LogicalRect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

pub(super) fn app_content_area(win: &AppWindow) -> LogicalRect {
    let size = win.window().size();
    let scale = win.window().scale_factor().max(0.01) as f32;
    let mut area = LogicalRect {
        x: 0.0,
        y: if win.get_custom_titlebar() {
            38.0
        } else if win.get_is_mac() {
            28.0
        } else {
            0.0
        },
        w: size.width as f32 / scale,
        h: 0.0,
    };
    area.h = size.height as f32 / scale - area.y;

    if win.get_zen_mode() {
        return area;
    }

    if win.get_welcome_as_sidebar() {
        let dock = win.get_welcome_sidebar_dock().to_string();
        let sidebar_strip_outside = !win.get_welcome_collapsed()
            && win.get_sidebar_collapsed()
            && win.get_sidebar_dock().as_str() == dock.as_str();
        let welcome_taken = (if win.get_welcome_collapsed() {
            36.0
        } else {
            win.get_welcome_sidebar_width()
        }) + if sidebar_strip_outside { 36.0 } else { 0.0 };
        shrink_edge(
            &mut area.x,
            &mut area.y,
            &mut area.w,
            &mut area.h,
            &dock,
            welcome_taken,
        );
    }

    let side_dock = win.get_sidebar_dock().to_string();
    let side_take = if win.get_sidebar_collapsed() {
        36.0
    } else if side_dock == "left" || side_dock == "right" {
        win.get_sidebar_width() + 4.0
    } else {
        win.get_sidebar_height() + 4.0
    };
    shrink_edge(
        &mut area.x,
        &mut area.y,
        &mut area.w,
        &mut area.h,
        &side_dock,
        side_take,
    );
    if win.get_quick_panel_open() {
        let quick_dock = win.get_quick_panel_dock().to_string();
        let quick_merged = win.get_quick_panel_collapsed()
            && ((win.get_welcome_as_sidebar()
                && win.get_welcome_collapsed()
                && win.get_welcome_sidebar_dock().as_str() == quick_dock.as_str())
                || (win.get_sidebar_collapsed() && side_dock.as_str() == quick_dock.as_str()));
        if quick_merged {
            return area;
        }
        let quick_take = if win.get_quick_panel_collapsed() {
            36.0
        } else if quick_dock == "left" || quick_dock == "right" {
            win.get_quick_panel_width() + 4.0
        } else {
            win.get_quick_panel_height() + 4.0
        };
        shrink_edge(
            &mut area.x,
            &mut area.y,
            &mut area.w,
            &mut area.h,
            &quick_dock,
            quick_take,
        );
    }
    area
}

pub(super) fn active_terminal_panel_rects(
    win: &AppWindow,
) -> Option<(String, LogicalRect, TerminalState)> {
    let active = win.get_active_tab_id().to_string();
    if active.is_empty() || active == "welcome" {
        return None;
    }

    let area = app_content_area(win);
    let panes = win.get_panes();
    let pane = (0..panes.row_count())
        .filter_map(|i| panes.row_data(i))
        .find(|p| p.active_id.as_str() == active.as_str())?;

    let terms = win.get_terminals();
    let term_state = (0..terms.row_count())
        .filter_map(|i| terms.row_data(i))
        .find(|t| t.id.as_str() == active.as_str())?;

    Some((
        active,
        LogicalRect {
            x: area.x + pane.x,
            y: area.y + pane.y + 40.0,
            w: pane.w,
            h: (pane.h - 40.0).max(0.0),
        },
        term_state,
    ))
}

/// The SFTP file-list rectangle inside the active terminal panel. Kept for a
/// future dedicated SFTP drop target; the shell-page drop currently accepts the
/// whole terminal panel instead of just this region (#drag-onto-shell).
#[allow(dead_code)]
pub(super) fn active_sftp_file_list_rect(win: &AppWindow) -> Option<LogicalRect> {
    let (_active, term, term_state) = active_terminal_panel_rects(win)?;
    // Focus mode hides SFTP on every tab; the stored collapse flag stays put.
    if !win.get_sftp_enabled() || win.get_zen_mode() || term_state.sftp_collapsed {
        return None;
    }

    // TerminalView starts with a 24px connection-status line (hidden in zen
    // mode); SFTP docks inside the remaining dock-region. This mirrors
    // ui/terminal_view.slint.
    let strip = if win.get_zen_mode() { 0.0 } else { 24.0 };
    let dock_region = LogicalRect {
        x: term.x,
        y: term.y + strip,
        w: term.w,
        h: (term.h - strip).max(0.0),
    };
    let dock = win.get_sftp_dock().to_string();
    let mut panel = LogicalRect {
        x: dock_region.x,
        y: dock_region.y,
        w: if dock == "left" || dock == "right" {
            term_state.sftp_panel_width
        } else {
            dock_region.w
        },
        h: if dock == "left" || dock == "right" {
            dock_region.h
        } else {
            term_state.sftp_panel_height
        },
    };
    if dock == "right" {
        panel.x = dock_region.x + (dock_region.w - panel.w).max(0.0);
    } else if dock == "bottom" {
        panel.y = dock_region.y + (dock_region.h - panel.h).max(0.0);
    }

    // SftpPanel layout: toolbar 34, then file headers 20 + separator 1; when the
    // tree is shown (top/bottom docks), the file list starts after tree 160 + sep.
    let show_tree = dock != "left" && dock != "right";
    panel.y += 34.0 + 20.0 + 1.0;
    panel.h = (panel.h - 34.0 - 20.0 - 1.0).max(0.0);
    if show_tree {
        panel.x += 160.0 + 1.0;
        panel.w = (panel.w - 160.0 - 1.0).max(0.0);
    }
    Some(panel)
}

/// Thickness the SFTP panel occupies inside a terminal.
/// Focus mode (`zen`) hides the panel and its collapsed strip, on every tab.
pub(crate) fn sftp_panel_take(enabled: bool, zen: bool, collapsed: bool, panel: f32) -> f32 {
    if !enabled || zen {
        0.0
    } else if collapsed {
        36.0
    } else {
        panel + 4.0
    }
}

/// Focus mode hides the quick-command panel and the SFTP panel together.
/// The decision does not depend on which session tab is active.
pub(crate) fn focus_hides_quick_and_sftp(zen: bool) -> bool {
    zen
}

#[cfg(test)]
mod focus_panel_tests {
    use super::{focus_hides_quick_and_sftp, sftp_panel_take};

    #[test]
    fn focus_mode_hides_sftp_and_quick_panels_across_tabs() {
        assert!(focus_hides_quick_and_sftp(true));
        assert!(!focus_hides_quick_and_sftp(false));
        // Expanded and collapsed tabs both lose the panel while focus is on.
        assert_eq!(sftp_panel_take(true, true, false, 380.0), 0.0);
        assert_eq!(sftp_panel_take(true, true, true, 380.0), 0.0);
        // Turning focus off restores the tab's own state.
        assert_eq!(sftp_panel_take(true, false, true, 380.0), 36.0);
        assert_eq!(sftp_panel_take(true, false, false, 380.0), 384.0);
        // The SFTP master switch hides the panel even when focus mode is off.
        assert_eq!(sftp_panel_take(false, false, false, 380.0), 0.0);
        assert_eq!(sftp_panel_take(false, true, false, 380.0), 0.0);
    }
}
