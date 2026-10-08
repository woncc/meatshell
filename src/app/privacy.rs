//! Screenshot redaction for SSH usernames, hosts, and ports.
//!
//! The saved session and the live connection keep the real address. This only
//! rewrites strings the window draws (status line, sidebar, tab title, session
//! list) while the eye toggle is closed. Each identity token becomes a fixed
//! `****`, so the label does not keep `@`, `:port`, or the original length.

use super::*;

pub(super) const IDENTITY_MASK: &str = "****";

pub(super) fn displayed_identity_text(hide: bool, raw: &str, user: &str, host: &str) -> String {
    if hide {
        redact_connection_text(raw, user, host)
    } else {
        raw.to_string()
    }
}

/// Replace `user@host[:port]`, other `name@host[:port]` tokens, IPv4
/// addresses (with a trailing port), and the known user/host words with one
/// fixed `****`. Surrounding status words stay put.
pub(super) fn redact_connection_text(text: &str, user: &str, host: &str) -> String {
    let user = user.trim();
    let host = host.trim();
    let mut out = text.to_string();
    if !user.is_empty() && !host.is_empty() {
        out = replace_bounded(&out, &format!("{user}@{host}"), IDENTITY_MASK, true);
    }
    out = mask_at_pairs(&out);
    out = mask_ipv4(&out);
    if maskable_host(host) {
        out = replace_bounded(&out, host, IDENTITY_MASK, true);
    }
    if user.len() >= 2 {
        out = replace_bounded(&out, user, IDENTITY_MASK, false);
    }
    out
}

/// User and host encoded in a sidebar label (`alice@10.0.0.1`, or a telnet
/// label that has no `@` and falls back to the probe host).
pub(super) fn identity_from_connection_label(
    label: &str,
    user: &str,
    probe_host: &str,
) -> (String, String) {
    if let Some((left, right)) = label.rsplit_once('@') {
        let left = left.trim();
        let right = right.trim();
        if !left.is_empty() && !right.is_empty() && !left.contains(' ') && !right.contains(' ') {
            return (left.to_string(), right.to_string());
        }
    }
    (user.trim().to_string(), probe_host.trim().to_string())
}

pub(super) fn write_terminal_status(row: &mut TerminalState, hide: bool, raw: &str) {
    let shown = displayed_identity_text(
        hide,
        raw,
        row.identity_user.as_str(),
        row.identity_host.as_str(),
    );
    row.status_raw = raw.into();
    row.status = shown.into();
}

pub(super) fn write_tab_title(row: &mut TabInfo, hide: bool, raw: &str) {
    let shown = displayed_identity_text(
        hide,
        raw,
        row.identity_user.as_str(),
        row.identity_host.as_str(),
    );
    row.title_raw = raw.into();
    row.title = shown.clone().into();
    row.title_len = tab_title_len(&shown);
}

pub(super) fn reapply_identity_mask(win: &AppWindow) {
    let hide = win.get_hide_ssh_identity();
    reapply_terminal_status(win, hide);
    reapply_tab_titles(win, hide);
}

fn reapply_terminal_status(win: &AppWindow, hide: bool) {
    let terminals = win.get_terminals();
    let Some(model) = terminals.as_any().downcast_ref::<VecModel<TerminalState>>() else {
        return;
    };
    for i in 0..model.row_count() {
        let Some(mut row) = model.row_data(i) else {
            continue;
        };
        let raw = if row.status_raw.is_empty() {
            row.status.to_string()
        } else {
            row.status_raw.to_string()
        };
        write_terminal_status(&mut row, hide, &raw);
        model.set_row_data(i, row);
    }
}

fn reapply_tab_titles(win: &AppWindow, hide: bool) {
    let apply = |row: &mut TabInfo| {
        let raw = if row.title_raw.is_empty() {
            row.title.to_string()
        } else {
            row.title_raw.to_string()
        };
        write_tab_title(row, hide, &raw);
    };
    let tabs = win.get_tabs();
    if let Some(model) = tabs.as_any().downcast_ref::<VecModel<TabInfo>>() {
        for i in 0..model.row_count() {
            if let Some(mut row) = model.row_data(i) {
                apply(&mut row);
                model.set_row_data(i, row);
            }
        }
    }
    let panes_rc = win.get_panes();
    let Some(panes) = panes_rc.as_any().downcast_ref::<VecModel<PaneInfo>>() else {
        return;
    };
    for pi in 0..panes.row_count() {
        let Some(pane) = panes.row_data(pi) else {
            continue;
        };
        let Some(tabs) = pane.tabs.as_any().downcast_ref::<VecModel<TabInfo>>() else {
            continue;
        };
        for ti in 0..tabs.row_count() {
            if let Some(mut row) = tabs.row_data(ti) {
                apply(&mut row);
                tabs.set_row_data(ti, row);
            }
        }
    }
}

fn maskable_host(host: &str) -> bool {
    host.len() >= 2 && host.chars().any(|c| !c.is_ascii_digit())
}

fn is_host_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_'
}

fn replace_bounded(text: &str, needle: &str, replacement: &str, swallow_port: bool) -> String {
    if needle.is_empty() || !text.contains(needle) {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let needle: Vec<char> = needle.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let mut end = i + needle.len();
        let matches = end <= chars.len() && chars[i..end] == needle[..];
        let bounded = matches
            && (i == 0 || !is_host_char(chars[i - 1]))
            && (end == chars.len() || !is_host_char(chars[end]));
        if bounded {
            if swallow_port {
                end = port_suffix_end(&chars, end);
            }
            out.push_str(replacement);
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// A `:port` glued to a host (`:22`, `:63322`). The port is part of the
/// identity, so the mask swallows it instead of leaving the number on screen.
fn port_suffix_end(chars: &[char], end: usize) -> usize {
    if end >= chars.len() || chars[end] != ':' {
        return end;
    }
    let mut pos = end + 1;
    let start = pos;
    while pos < chars.len() && chars[pos].is_ascii_digit() && pos - start < 5 {
        pos += 1;
    }
    let digits = pos - start;
    if digits == 0 {
        return end;
    }
    if pos < chars.len() && (chars[pos].is_ascii_digit() || is_host_char(chars[pos])) {
        return end;
    }
    let value: u32 = chars[start..pos]
        .iter()
        .collect::<String>()
        .parse()
        .unwrap_or(u32::MAX);
    if (1..=65535).contains(&value) {
        pos
    } else {
        end
    }
}

fn mask_at_pairs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut ranges = Vec::new();
    for i in 0..chars.len() {
        if chars[i] != '@' {
            continue;
        }
        let mut left = i;
        while left > 0 && is_host_char(chars[left - 1]) {
            left -= 1;
        }
        let mut right = i + 1;
        while right < chars.len() && is_host_char(chars[right]) {
            right += 1;
        }
        if left < i && right > i + 1 {
            ranges.push((left, port_suffix_end(&chars, right)));
        }
    }
    if ranges.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut cursor = 0;
    for (left, right) in ranges {
        if left < cursor {
            continue;
        }
        out.extend(chars[cursor..left].iter());
        out.push_str(IDENTITY_MASK);
        cursor = right;
    }
    out.extend(chars[cursor..].iter());
    out
}

fn mask_ipv4(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = ipv4_at(&chars, i) {
            out.push_str(IDENTITY_MASK);
            i = port_suffix_end(&chars, end);
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn ipv4_at(chars: &[char], start: usize) -> Option<usize> {
    if start > 0 && is_host_char(chars[start - 1]) {
        return None;
    }
    let mut pos = start;
    for octet in 0..4 {
        if octet > 0 {
            if pos >= chars.len() || chars[pos] != '.' {
                return None;
            }
            pos += 1;
        }
        let octet_start = pos;
        while pos < chars.len() && chars[pos].is_ascii_digit() {
            pos += 1;
        }
        let digits = pos - octet_start;
        if !(1..=3).contains(&digits) {
            return None;
        }
        let value: u16 = chars[octet_start..pos]
            .iter()
            .collect::<String>()
            .parse()
            .ok()?;
        if value > 255 {
            return None;
        }
    }
    if pos < chars.len() && is_host_char(chars[pos]) {
        return None;
    }
    Some(pos)
}

#[cfg(test)]
mod tests {
    use super::redact_connection_text;

    #[test]
    fn masks_user_host_ipv4_and_jump_without_eating_words() {
        let connected = redact_connection_text("Connected alice@10.0.0.5", "alice", "10.0.0.5");
        assert_eq!(connected, "Connected ****");
        assert!(
            !connected.contains("alice") && !connected.contains("10.0.0.5"),
            "{connected}"
        );

        let handshake =
            redact_connection_text("SSH handshake to db.example:63322", "alice", "db.example");
        assert_eq!(handshake, "SSH handshake to ****");
        assert!(!handshake.contains("63322"), "{handshake}");
        assert!(!handshake.contains('@'), "{handshake}");

        let listed = redact_connection_text("root@10.0.0.5:63322", "root", "10.0.0.5");
        assert_eq!(listed, "****");
        let status = redact_connection_text("SSH 握手 10.0.0.5:63322", "root", "10.0.0.5");
        assert_eq!(status, "SSH 握手 ****");

        let jump = redact_connection_text(
            "via jump host bob@bastion:2222 -> 10.1.1.9:22",
            "alice",
            "10.1.1.9",
        );
        assert_eq!(jump, "via jump host **** -> ****");
        assert!(!jump.contains("2222") && !jump.contains(":22"), "{jump}");

        let plain = redact_connection_text("Connecting...", "alice", "10.0.0.5");
        assert_eq!(plain, "Connecting...");

        let word = redact_connection_text("root cause stays", "root", "10.0.0.5");
        // "root" is its own word, so the username is masked, but the rest stays.
        assert_eq!(word, "**** cause stays");
        let inside = redact_connection_text("rootkit stays", "root", "10.0.0.5");
        assert_eq!(inside, "rootkit stays");
    }

    #[test]
    fn serial_label_and_empty_identity_stay() {
        assert_eq!(
            redact_connection_text("Connected COM3 @ 9600", "", ""),
            "Connected COM3 @ 9600"
        );
        assert_eq!(
            redact_connection_text("Not connected", "", ""),
            "Not connected"
        );
    }
}
