use super::*;

fn hist_line(s: &str) -> Line {
    (s.to_string(), Vec::new(), false)
}

fn wrapped_hist_line(s: &str) -> Line {
    (s.to_string(), Vec::new(), true)
}

fn make_buf(
    rows: u16,
    cols: u16,
    history: &[&str],
    live_lines: &[&str],
    view_offset: usize,
) -> TermBuffer {
    let mut parser = vt100::Parser::new(rows, cols, 0);
    parser.process(live_lines.join("\r\n").as_bytes());
    TermBuffer {
        parser,
        find_query: String::new(),
        is_dark: false,
        output_highlight: OutputHighlightPreset::Log,
        custom_highlight_rules: Vec::new(),
        json_format_output: false,
        vt100_drawing: true,
        charset: crate::terminal::CharsetTracker::default(),
        interactive_echo_until: std::time::Instant::now(),
        sel_anchor: None,
        sel_focus: None,
        sel_ranges: Vec::new(),
        mouse_tracked: false,
        history: history.iter().map(|s| hist_line(s)).collect(),
        prev: Vec::new(),
        view_offset,
        scroll_accum: 0.0,
        displayed_text: Vec::new(),
        csi_state: CsiState::Normal,
        csi_pending: Vec::new(),
        raw: std::collections::VecDeque::new(),
        suppress_alt_erase_saved: false,
        primary_fullscreen: false,
        cursor_hidden: false,
        session_log: None,
        session_log_spec: None,
    }
}

#[test]
fn settings_modal_yields_macos_wheel_to_its_own_scroll_view() {
    assert!(macos_terminal_wheel_can_target_terminal(false));
    assert!(!macos_terminal_wheel_can_target_terminal(true));
}

#[test]
fn releasing_scrollback_drops_retained_history_and_replay_bytes() {
    let mut buffer = make_buf(5, 20, &["old-1", "old-2"], &["live"], 2);
    buffer.prev = vec![hist_line("previous")];
    buffer.raw.extend([1, 2, 3, 4]);
    buffer.displayed_text.push("visible".to_string());
    buffer.sel_anchor = Some((0, 0));
    buffer.sel_focus = Some((1, 1));
    buffer.sel_ranges.push(((0, 0), (1, 1)));

    buffer.release_scrollback();

    assert!(buffer.history.is_empty());
    assert_eq!(buffer.history.capacity(), 0);
    assert!(buffer.prev.is_empty());
    assert_eq!(buffer.prev.capacity(), 0);
    assert!(buffer.raw.is_empty());
    assert_eq!(buffer.raw.capacity(), 0);
    assert!(buffer.displayed_text.is_empty());
    assert!(buffer.sel_anchor.is_none());
    assert!(buffer.sel_focus.is_none());
    assert!(buffer.sel_ranges.is_empty());
    assert_eq!(buffer.view_offset, 0);
}

#[test]
fn disconnect_can_keep_rendered_history_and_still_drops_the_raw_stream() {
    let mut kept = make_buf(4, 20, &["earlier"], &["visible"], 0);
    kept.raw.push_back(b'x');
    kept.release_history_keep_screen(true);
    assert_eq!(kept.history.len(), 1);
    assert_eq!(kept.history[0].0, "earlier");
    assert!(kept.raw.is_empty());
    assert_eq!(kept.view_offset, 0);

    let mut dropped = make_buf(4, 20, &["earlier"], &["visible"], 2);
    dropped.raw.push_back(b'y');
    dropped.release_history_keep_screen(false);
    assert!(dropped.history.is_empty());
    assert!(dropped.raw.is_empty());
    assert_eq!(dropped.view_offset, 0);
}

#[test]
fn scrollback_navigation_keys_only_handle_a_scrolled_normal_screen() {
    let mut buffer = make_buf(
        3,
        20,
        &["old-1", "old-2", "old-3", "old-4", "old-5"],
        &["live"],
        2,
    );

    assert!(handle_scrollback_key(&mut buffer, ScrollbackKey::PageUp));
    assert_eq!(buffer.view_offset, 5);
    assert!(handle_scrollback_key(&mut buffer, ScrollbackKey::PageDown));
    assert_eq!(buffer.view_offset, 2);
    assert!(handle_scrollback_key(&mut buffer, ScrollbackKey::Home));
    assert_eq!(buffer.view_offset, 5);
    assert!(handle_scrollback_key(&mut buffer, ScrollbackKey::End));
    assert_eq!(buffer.view_offset, 0);

    assert!(!handle_scrollback_key(&mut buffer, ScrollbackKey::PageUp));

    buffer.parser.process(b"\x1b[?1049h");
    buffer.view_offset = 1;
    assert!(!handle_scrollback_key(&mut buffer, ScrollbackKey::PageDown));
    assert_eq!(buffer.view_offset, 1);
}

mod charset;
mod colors;
mod protocol;
mod selection;
mod sftp_sorting;

#[test]
fn enabling_session_log_mid_session_seeds_screen_and_keeps_prompt_spacing() {
    let dir = std::env::temp_dir().join(format!("meatshell-log-test-{}", uuid::Uuid::new_v4()));
    let mut buffer = make_buf(5, 40, &[], &[], 0);
    buffer.parser.process(b"line one\r\nuser@host:~$ ");
    buffer.session_log_spec = Some(crate::terminal::SessionLogSpec {
        name: "web".into(),
        target: "ssh root@h:22".into(),
        mode: crate::config::SessionLogMode::Default,
    });

    // Global off: nothing is opened.
    buffer.apply_session_log(false, &dir).unwrap();
    assert!(buffer.session_log.is_none());

    // Turning it on for an open tab starts logging right away.
    buffer.apply_session_log(true, &dir).unwrap();
    let path = buffer.session_log.as_ref().unwrap().path().to_path_buf();
    let _ = buffer.ingest(b"ls\r\n");

    // Turning it off again closes the file.
    buffer.apply_session_log(false, &dir).unwrap();
    assert!(buffer.session_log.is_none());

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.lines().any(|l| l.ends_with("] line one")), "{text}");
    assert!(
        text.lines().any(|l| l.ends_with("] user@host:~$ ls")),
        "{text}"
    );
    assert!(text
        .lines()
        .last()
        .unwrap()
        .starts_with("=== session log closed"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn session_override_off_wins_over_global_on() {
    let dir = std::env::temp_dir().join(format!("meatshell-log-test-{}", uuid::Uuid::new_v4()));
    let mut buffer = make_buf(5, 40, &[], &[], 0);
    buffer.session_log_spec = Some(crate::terminal::SessionLogSpec {
        name: "web".into(),
        target: "local".into(),
        mode: crate::config::SessionLogMode::Off,
    });
    buffer.apply_session_log(true, &dir).unwrap();
    assert!(buffer.session_log.is_none());
    assert!(!dir.exists());
}

fn history_text(buffer: &TermBuffer) -> Vec<String> {
    buffer
        .history
        .iter()
        .map(|(text, _, _)| text.clone())
        .collect()
}

fn ingest_scrolled_lines(buffer: &mut TermBuffer) {
    let _ = buffer.ingest(b"alpha\r\nbeta\r\ngamma\r\ndelta\r\nepsilon\r\n");
    assert!(
        history_text(buffer)
            .iter()
            .any(|line| line.contains("alpha")),
        "static output should enter scrollback: {:?}",
        history_text(buffer)
    );
}

#[test]
fn alt_screen_round_trip_keeps_prior_scrollback() {
    let mut buffer = make_buf(4, 40, &[], &[], 0);
    ingest_scrolled_lines(&mut buffer);
    let _ = buffer.ingest(b"\x1b[?1049hTOP\x1b[H\x1b[2JTOP2");
    let _ = buffer.ingest(b"\x1b[?1049l");
    let _ = buffer.ingest(b"after\r\n");
    assert!(
        history_text(&buffer)
            .iter()
            .any(|line| line.contains("alpha")),
        "history lost after alt screen: {:?}",
        history_text(&buffer)
    );
}

#[test]
fn ncurses_rmcup_erase_saved_does_not_drop_scrollback() {
    let mut buffer = make_buf(4, 40, &[], &[], 0);
    ingest_scrolled_lines(&mut buffer);
    let _ = buffer.ingest(b"\x1b[?1049hTOP");
    let _ = buffer.ingest(b"\x1b[?1049l\x1b[3J");
    let _ = buffer.ingest(b"after\r\n");
    assert!(
        history_text(&buffer)
            .iter()
            .any(|line| line.contains("alpha")),
        "rmcup CSI 3 J cleared history: {:?}",
        history_text(&buffer)
    );
}

#[test]
fn erase_saved_split_after_alt_exit_does_not_drop_scrollback() {
    let mut buffer = make_buf(4, 40, &[], &[], 0);
    ingest_scrolled_lines(&mut buffer);
    let _ = buffer.ingest(b"\x1b[?1049hTOP");
    let _ = buffer.ingest(b"\x1b[?1049l");
    let _ = buffer.ingest(b"\x1b[3J");
    let _ = buffer.ingest(b"after\r\n");
    assert!(
        history_text(&buffer)
            .iter()
            .any(|line| line.contains("alpha")),
        "split CSI 3 J cleared history: {:?}",
        history_text(&buffer)
    );
}

#[test]
fn explicit_erase_saved_on_primary_still_clears_scrollback() {
    let mut buffer = make_buf(4, 40, &[], &[], 0);
    ingest_scrolled_lines(&mut buffer);
    let _ = buffer.ingest(b"\x1b[3J");
    assert!(
        history_text(&buffer).is_empty(),
        "primary CSI 3 J should clear history: {:?}",
        history_text(&buffer)
    );
}

/// #490: shrinking the window while an alternate-screen program (nano, vim)
/// runs must not leave the cursor saved by `CSI ? 1049 h` outside the grid,
/// or the first character printed after `CSI ? 1049 l` panics inside vt100.
/// vt100 0.16.2 clamps the saved cursor on resize; keep this when upgrading.
#[test]
fn leaving_alternate_screen_after_shrinking_keeps_cursor_in_bounds() {
    for (start, rows, cols) in [("\x1b[40;1H", 20, 120), ("\x1b[40;100H", 20, 60)] {
        let mut parser = vt100::Parser::new(40, 120, 0);
        parser.process(start.as_bytes());
        parser.process(b"\x1b[?1049h");
        parser.screen_mut().set_size(rows, cols);
        parser.process(b"\x1b[?1049l");
        parser.process("x\u{4e2d}".as_bytes());
        let (row, col) = parser.screen().cursor_position();
        assert!(
            row < rows && col <= cols,
            "cursor {row},{col} outside {rows}x{cols}"
        );
    }
}

fn numbered_lines(count: usize) -> Vec<u8> {
    (1..=count)
        .map(|n| format!("LINE-{n:02} prior-command\r\n"))
        .collect::<String>()
        .into_bytes()
}

fn history_has(buffer: &TermBuffer, needle: &str) -> bool {
    history_text(buffer)
        .iter()
        .any(|line| line.contains(needle))
}

fn live_has(buffer: &TermBuffer) -> String {
    buffer.parser.screen().contents()
}

/// procps `top` on TERM=xterm-256color in this environment does not emit
/// smcup. It hides the cursor and erases the primary display (`CSI H` +
/// `CSI 2 J`), then redraws. Ctrl+C shows the cursor again.
const TOP_ENTER: &[u8] = b"\x1b[?1h\x1b=\x1b[?25l\x1b[H\x1b[2JTOP-FRAME-1\r\nload\r\n";
const TOP_REDRAW: &[u8] = b"\x1b[H\x1b[2JTOP-FRAME-2\r\nload\r\n";
const TOP_CTRL_C: &[u8] = b"\x1b[?1l\x1b>\x1b[8;1H\r\n\x1b[?12l\x1b[?25h\x1b[K";

#[test]
fn primary_top_keeps_prior_output_and_drops_later_frames() {
    let mut buffer = make_buf(8, 80, &[], &[], 0);
    let _ = buffer.ingest(&numbered_lines(20));
    let _ = buffer.ingest(TOP_ENTER);
    let _ = buffer.ingest(TOP_REDRAW);
    let _ = buffer.ingest(TOP_CTRL_C);
    for n in 1..=20 {
        assert!(
            history_has(&buffer, &format!("LINE-{n:02}")),
            "missing LINE-{n:02} after primary top: {:?}",
            history_text(&buffer)
        );
    }
    let joined = history_text(&buffer).join("\n");
    assert!(
        !joined.contains("TOP-FRAME"),
        "top frame entered history: {joined}"
    );
    assert!(
        !live_has(&buffer).contains("TOP-FRAME-2") || !joined.contains("TOP-FRAME-2"),
        "redraw must not be stored as scrollback"
    );
}

#[test]
fn alt_screen_top_restores_visible_primary_lines() {
    let mut buffer = make_buf(8, 80, &[], &[], 0);
    let _ = buffer.ingest(&numbered_lines(20));
    let _ = buffer.ingest(b"\x1b[?1049h\x1b[H\x1b[2J\x1b[3JTOP-FRAME\r\n");
    assert!(buffer.parser.screen().alternate_screen());
    assert!(
        !history_has(&buffer, "TOP-FRAME"),
        "alt frame must not enter history"
    );
    let _ = buffer.ingest(b"\x1b[?1049l\x1b[3J");
    assert!(!buffer.parser.screen().alternate_screen());
    assert!(history_has(&buffer, "LINE-01"));
    assert!(
        live_has(&buffer).contains("LINE-20") || history_has(&buffer, "LINE-20"),
        "visible primary lines lost after rmcup: history={:?} live={}",
        history_text(&buffer),
        live_has(&buffer)
    );
    assert!(!history_has(&buffer, "TOP-FRAME"));
}

#[test]
fn reflow_after_primary_top_keeps_prior_lines() {
    let mut buffer = make_buf(8, 80, &[], &[], 0);
    let _ = buffer.ingest(&numbered_lines(20));
    let _ = buffer.ingest(TOP_ENTER);
    buffer.reflow(10, 40);
    assert!(history_has(&buffer, "LINE-01"));
    assert!(history_has(&buffer, "LINE-20"));
    assert!(!history_has(&buffer, "TOP-FRAME"));
}

#[test]
fn primary_clear_snapshots_once_then_resumes_scrollback() {
    let mut buffer = make_buf(8, 80, &[], &[], 0);
    let _ = buffer.ingest(&numbered_lines(20));
    let _ = buffer.ingest(b"\x1b[H\x1b[2J");
    let _ = buffer.ingest(&numbered_lines(10).join_as_after());
    assert!(history_has(&buffer, "LINE-01"));
    assert!(history_has(&buffer, "LINE-20"));
    assert!(
        history_has(&buffer, "AFTER-01"),
        "clear left history capture stuck: {:?}",
        history_text(&buffer)
    );
}

trait AfterLines {
    fn join_as_after(self) -> Vec<u8>;
}

impl AfterLines for Vec<u8> {
    fn join_as_after(self) -> Vec<u8> {
        String::from_utf8(self)
            .expect("numbered lines")
            .replace("LINE-", "AFTER-")
            .into_bytes()
    }
}

#[test]
fn primary_fullscreen_snapshot_still_respects_history_cap() {
    let mut buffer = make_buf(4, 20, &[], &[], 0);
    for n in 0..crate::terminal::MAX_HISTORY {
        buffer
            .history
            .push_back((format!("OLD-{n}"), Vec::new(), false));
    }
    let _ = buffer.ingest(b"KEEP-ME\r\n");
    let _ = buffer.ingest(b"\x1b[H\x1b[2J");
    assert_eq!(buffer.history.len(), crate::terminal::MAX_HISTORY);
    assert!(!history_text(&buffer).iter().any(|line| line == "OLD-0"));
    assert!(history_has(&buffer, "KEEP-ME"));
}
