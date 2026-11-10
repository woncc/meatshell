//! Local resource-panel metric: realtime speed, or ICMP latency.
//!
//! A brand-new config shows ICMP latency for the active SSH or Telnet host.
//! A saved `"speed"` or `"latency"` choice is left alone, and a config that
//! never stored the key stays on speed. The number is an ICMP echo round trip,
//! not an SSH protocol RTT. `--` means there is no remote host or ping did not
//! answer. Speed text is never reused for that reading.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::system::format_bytes_per_sec;
use super::system_types::SystemSnapshot;

const HISTORY_LEN: usize = super::system::SPARKLINE_HISTORY_LEN;
/// Drawn as an empty bar. It is not a measured millisecond.
const SAMPLE_GAP: f32 = -1.0;
/// No target, or the latency view is off: wait this long instead of polling.
pub(crate) const PROBE_IDLE: Duration = Duration::from_secs(1);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Per-window local sample plus the latency probe that feeds the optional mode.
pub(crate) struct LocalMachine {
    pub(crate) snap: Mutex<SystemSnapshot>,
    pub(crate) latency: LatencyProbe,
}

pub(crate) type LocalSnap = Arc<LocalMachine>;

impl LocalMachine {
    pub(crate) fn new() -> Self {
        Self {
            snap: Mutex::new(SystemSnapshot::default()),
            latency: LatencyProbe::spawn(),
        }
    }
}

/// What the lower local panel should show. Speed and latency never share a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalMetricView {
    pub(crate) latency_mode: bool,
    pub(crate) speed_up: String,
    pub(crate) speed_down: String,
    pub(crate) latency_text: String,
}

pub(crate) fn local_metric_view(
    latency_mode: bool,
    tx_per_sec: u64,
    rx_per_sec: u64,
    rtt_ms: Option<f32>,
) -> LocalMetricView {
    if latency_mode {
        LocalMetricView {
            latency_mode: true,
            speed_up: String::new(),
            speed_down: String::new(),
            latency_text: format_latency_ms(rtt_ms),
        }
    } else {
        LocalMetricView {
            latency_mode: false,
            speed_up: format_bytes_per_sec(tx_per_sec),
            speed_down: format_bytes_per_sec(rx_per_sec),
            latency_text: String::new(),
        }
    }
}

/// `None`, non-finite, or negative becomes `--`. Sub-millisecond is `<1 ms`.
pub(crate) fn format_latency_ms(ms: Option<f32>) -> String {
    match ms {
        Some(ms) if ms.is_finite() && (0.0..1.0).contains(&ms) => "<1 ms".to_string(),
        Some(ms) if ms.is_finite() && ms >= 0.0 => format!("{} ms", ms.round() as u32),
        _ => "--".to_string(),
    }
}

/// Hostnames and addresses safe to pass as one ping argument. Leading dashes
/// and shell metacharacters are rejected so the probe cannot be turned into flags.
pub(crate) fn ping_host(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let host = trimmed
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(trimmed);
    if host.is_empty() || host.len() > 253 || host.starts_with('-') {
        return None;
    }
    if !host
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b':' | b'-' | b'_'))
    {
        return None;
    }
    Some(host)
}

pub(crate) struct PingSpec {
    pub(crate) program: &'static str,
    pub(crate) args: Vec<String>,
}

pub(crate) fn ping_spec(host: &str) -> Option<PingSpec> {
    let host = ping_host(host)?;
    let v6 = host.contains(':');
    #[cfg(windows)]
    {
        let mut args = Vec::with_capacity(6);
        if v6 {
            args.push("-6".to_string());
        }
        args.extend([
            "-n".into(),
            "1".into(),
            "-w".into(),
            "1000".into(),
            host.into(),
        ]);
        Some(PingSpec {
            program: "ping",
            args,
        })
    }
    #[cfg(target_os = "macos")]
    {
        let mut args = vec![
            "-n".into(),
            "-c".into(),
            "1".into(),
            "-W".into(),
            "1000".into(),
        ];
        if v6 {
            args.insert(0, "-6".into());
        }
        args.push(host.into());
        Some(PingSpec {
            program: "ping",
            args,
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut args = vec![
            "-n".into(),
            "-c".into(),
            "1".into(),
            "-W".into(),
            "1".into(),
        ];
        if v6 {
            args.insert(0, "-6".into());
        }
        args.push(host.into());
        Some(PingSpec {
            program: "ping",
            args,
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (host, v6);
        None
    }
}

/// First `time=12.4 ms`, `time<1ms`, or `=12ms` in ping output.
pub(crate) fn parse_ping_rtt_ms(output: &str) -> Option<f32> {
    let lower = output.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'm' && bytes[i + 1] == b's' {
            let mut end = i;
            while end > 0 && bytes[end - 1].is_ascii_whitespace() {
                end -= 1;
            }
            let mut start = end;
            while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
                start -= 1;
            }
            if start < end && start > 0 {
                let mark = bytes[start - 1];
                if mark == b'=' || mark == b'<' {
                    if let Ok(value) = lower[start..end].parse::<f32>() {
                        if value.is_finite() && value >= 0.0 {
                            return Some(if mark == b'<' && value <= 1.0 {
                                0.5
                            } else {
                                value
                            });
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

struct ProbeState {
    target: String,
    probing: bool,
    generation: u64,
    latest: Option<f32>,
    history: Vec<f32>,
}

impl ProbeState {
    fn new() -> Self {
        Self {
            target: String::new(),
            probing: false,
            generation: 0,
            latest: None,
            history: Vec::new(),
        }
    }

    /// Changing host drops that host's samples. The same host keeps them, so a
    /// speed/latency toggle is not treated as a new target.
    fn set_target(&mut self, host: &str) {
        let host = ping_host(host).unwrap_or("");
        if self.target == host {
            return;
        }
        self.target = host.to_string();
        self.generation = self.generation.wrapping_add(1);
        self.latest = None;
        self.history.clear();
    }

    fn set_probing(&mut self, on: bool) {
        self.probing = on;
    }

    fn record(&mut self, generation: u64, host: &str, rtt: Option<f32>) {
        if self.generation != generation || self.target != host {
            return;
        }
        self.latest = rtt.filter(|ms| ms.is_finite() && *ms >= 0.0);
        let point = self.latest.unwrap_or(SAMPLE_GAP);
        if self.history.len() >= HISTORY_LEN {
            self.history.remove(0);
        }
        self.history.push(point);
    }
}

pub(crate) struct LatencyProbe {
    stop: Arc<AtomicBool>,
    data: Arc<Mutex<ProbeState>>,
}

impl LatencyProbe {
    pub(crate) fn spawn() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let data = Arc::new(Mutex::new(ProbeState::new()));
        let stop_thread = Arc::clone(&stop);
        let data_thread = Arc::clone(&data);
        let _ = thread::Builder::new()
            .name("latency-probe".into())
            .spawn(move || probe_loop(stop_thread, data_thread));
        Self { stop, data }
    }

    pub(crate) fn set_target(&self, host: &str) {
        if let Ok(mut data) = self.data.lock() {
            data.set_target(host);
        }
    }

    /// Latency view probes. Speed view does not. This does not clear history.
    pub(crate) fn set_probing(&self, on: bool) {
        if let Ok(mut data) = self.data.lock() {
            data.set_probing(on);
        }
    }

    pub(crate) fn latest(&self) -> Option<f32> {
        self.data.lock().ok().and_then(|data| data.latest)
    }

    pub(crate) fn history(&self) -> Vec<f32> {
        self.data
            .lock()
            .map(|data| data.history.clone())
            .unwrap_or_default()
    }
}

impl Drop for LatencyProbe {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Where the local sampler is relative to the window. These are not the same
/// event: disconnect clears the ICMP target, and this enum does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelPresence {
    /// Focused and the resource panel is on screen.
    Active,
    /// Visible, but another window is focused.
    Unfocused,
    /// Minimized or occluded. Not a disconnect.
    Hidden,
    /// The resource sidebar is collapsed.
    SidebarCollapsed,
    /// Focus mode hides the panel.
    FocusMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalSampleDecision {
    pub(crate) sample: bool,
    pub(crate) draw: bool,
}

/// Local sysinfo cadence. `continuous` is the saved “后台持续监控” switch.
/// It does not start ICMP, and it does not override a collapsed sidebar or
/// focus mode. Unfocused without the switch samples about every fifth 1s tick.
pub(crate) fn local_sample_decision(
    presence: PanelPresence,
    continuous: bool,
    unfocused_tick: u32,
) -> LocalSampleDecision {
    match presence {
        PanelPresence::SidebarCollapsed | PanelPresence::FocusMode => LocalSampleDecision {
            sample: false,
            draw: false,
        },
        PanelPresence::Active => LocalSampleDecision {
            sample: true,
            draw: true,
        },
        PanelPresence::Unfocused => LocalSampleDecision {
            sample: continuous || unfocused_tick % 5 == 0,
            draw: false,
        },
        PanelPresence::Hidden => LocalSampleDecision {
            sample: continuous,
            draw: false,
        },
    }
}

/// ICMP runs only for the latency view of a real host. Focus, hide, and the
/// background switch do not appear here: turning the switch off does not stop
/// an in-view probe, and turning it on does not probe while speed is showing.
pub(crate) fn icmp_probes(latency_view: bool, host_set: bool) -> bool {
    latency_view && host_set
}

fn probe_loop(stop: Arc<AtomicBool>, data: Arc<Mutex<ProbeState>>) {
    while !stop.load(Ordering::Relaxed) {
        let (host, generation, probing) = match data.lock() {
            Ok(state) => (state.target.clone(), state.generation, state.probing),
            Err(_) => return,
        };
        // Hidden, unfocused, and the background-monitor switch do not gate this
        // loop. ICMP runs only while the latency view has a real target.
        if host.is_empty() || !probing {
            sleep_flag(PROBE_IDLE, &stop);
            continue;
        }
        let started = Instant::now();
        let rtt = ping_once(&host);
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(mut state) = data.lock() {
            state.record(generation, &host, rtt);
        }
        let rest = Duration::from_secs(1).saturating_sub(started.elapsed());
        sleep_flag(rest, &stop);
    }
}

fn sleep_flag(total: Duration, stop: &AtomicBool) {
    let step = Duration::from_millis(50);
    let start = Instant::now();
    while start.elapsed() < total {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        thread::sleep(step.min(total.saturating_sub(start.elapsed())));
    }
}

fn ping_once(host: &str) -> Option<f32> {
    let spec = ping_spec(host)?;
    let mut command = Command::new(spec.program);
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= Duration::from_secs(2) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
    let mut buf = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_end(&mut buf);
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_end(&mut buf);
    }
    buf.truncate(8192);
    parse_ping_rtt_ms(&String::from_utf8_lossy(&buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_mode_keeps_throughput_units_and_hides_latency() {
        let view = local_metric_view(false, 0, 1536, Some(12.0));
        assert!(!view.latency_mode);
        assert_eq!(view.speed_up, "0 B/s");
        assert_eq!(view.speed_down, "1.5 KB/s");
        assert!(view.latency_text.is_empty());
        assert!(!view.speed_up.contains("ms"));
        assert!(!view.speed_down.contains("ms"));
    }

    #[test]
    fn latency_mode_is_milliseconds_not_throughput() {
        let shown = local_metric_view(true, 1536, 1536, Some(12.4));
        assert!(shown.latency_mode);
        assert!(shown.speed_up.is_empty());
        assert!(shown.speed_down.is_empty());
        assert_eq!(shown.latency_text, "12 ms");
        assert!(!shown.latency_text.contains("/s"));

        assert_eq!(format_latency_ms(Some(0.4)), "<1 ms");
        assert_eq!(format_latency_ms(None), "--");
        assert_eq!(format_latency_ms(Some(f32::NAN)), "--");
        assert_eq!(local_metric_view(true, 1, 1, None).latency_text, "--");
    }

    #[test]
    fn ping_output_parses_linux_windows_and_sub_millisecond() {
        assert_eq!(
            parse_ping_rtt_ms("64 bytes from 1.1.1.1: icmp_seq=1 ttl=57 time=12.4 ms"),
            Some(12.4)
        );
        assert_eq!(
            parse_ping_rtt_ms("Reply from 1.1.1.1: bytes=32 time=12ms TTL=54"),
            Some(12.0)
        );
        assert_eq!(parse_ping_rtt_ms("时间=15ms TTL=54"), Some(15.0));
        assert_eq!(parse_ping_rtt_ms("time<1ms"), Some(0.5));
        assert_eq!(parse_ping_rtt_ms("Request timed out."), None);
    }

    #[test]
    fn ping_host_rejects_flags_and_keeps_the_address_as_one_arg() {
        assert!(ping_host("-c").is_none());
        assert!(ping_host("host;rm").is_none());
        assert!(ping_host("bad host").is_none());
        assert!(ping_host("").is_none());
        assert_eq!(ping_host("  example.com "), Some("example.com"));
        assert_eq!(ping_host("[2001:db8::1]"), Some("2001:db8::1"));

        let spec = ping_spec("example.com").unwrap();
        assert_eq!(spec.program, "ping");
        assert_eq!(spec.args.last().map(String::as_str), Some("example.com"));
        assert_eq!(
            spec.args
                .iter()
                .filter(|arg| arg.as_str() == "example.com")
                .count(),
            1
        );
        assert!(spec.args.iter().all(|arg| !arg.contains(';')));

        let v6 = ping_spec("2001:db8::1").unwrap();
        assert!(v6.args.iter().any(|arg| arg == "-6"));
        assert_eq!(v6.args.last().map(String::as_str), Some("2001:db8::1"));
        assert!(ping_spec("-n").is_none());
    }

    #[test]
    fn latency_history_matches_the_widest_sidebar() {
        assert_eq!(
            super::HISTORY_LEN,
            super::super::system::SPARKLINE_HISTORY_LEN
        );
        assert!(super::HISTORY_LEN > 160);
    }

    #[test]
    fn stale_probe_does_not_overwrite_a_new_target() {
        let mut state = ProbeState::new();
        state.set_target("one.example");
        let first = state.generation;
        state.record(first, "one.example", Some(8.0));
        assert_eq!(state.latest, Some(8.0));
        assert_eq!(state.history, vec![8.0]);

        state.set_target("two.example");
        assert_eq!(state.latest, None);
        assert!(state.history.is_empty());
        state.record(first, "one.example", Some(99.0));
        assert_eq!(state.latest, None);
        state.record(state.generation, "two.example", None);
        assert_eq!(state.latest, None);
        assert_eq!(state.history, vec![SAMPLE_GAP]);
        state.record(state.generation, "two.example", Some(21.0));
        assert_eq!(state.latest, Some(21.0));
        assert_eq!(state.history, vec![SAMPLE_GAP, 21.0]);

        state.set_target("not a host");
        assert!(state.target.is_empty());
        assert_eq!(state.latest, None);
    }

    #[test]
    fn same_host_view_switch_keeps_limited_history() {
        let mut state = ProbeState::new();
        state.set_target("one.example");
        state.set_probing(true);
        let generation = state.generation;
        state.record(generation, "one.example", Some(4.0));
        state.record(generation, "one.example", None);
        state.set_probing(false);
        state.set_target("one.example");
        assert!(!state.probing);
        assert_eq!(state.history, vec![4.0, SAMPLE_GAP]);
        assert_eq!(state.latest, None);
        state.set_probing(true);
        assert_eq!(state.history, vec![4.0, SAMPLE_GAP]);
        state.set_target("other.example");
        assert!(state.history.is_empty());
        assert!(state.latest.is_none());
    }

    #[test]
    fn history_stays_inside_the_cap() {
        let mut state = ProbeState::new();
        state.set_target("one.example");
        for i in 0..(HISTORY_LEN + 3) {
            state.record(state.generation, "one.example", Some(i as f32));
        }
        assert_eq!(state.history.len(), HISTORY_LEN);
        assert_eq!(state.history[0], 3.0);
    }

    #[test]
    fn local_sampler_keeps_the_energy_pause_until_the_switch_is_on() {
        assert_eq!(PROBE_IDLE, Duration::from_secs(1));
        let off = |presence, tick| local_sample_decision(presence, false, tick);
        assert_eq!(
            off(PanelPresence::Active, 1),
            LocalSampleDecision {
                sample: true,
                draw: true
            }
        );
        assert!(!off(PanelPresence::Hidden, 1).sample);
        assert!(!off(PanelPresence::Unfocused, 1).sample);
        assert!(off(PanelPresence::Unfocused, 5).sample);
        assert!(!off(PanelPresence::Unfocused, 5).draw);
        assert!(!off(PanelPresence::SidebarCollapsed, 5).sample);
        assert!(!off(PanelPresence::FocusMode, 5).sample);

        let on = |presence| local_sample_decision(presence, true, 1);
        assert!(on(PanelPresence::Hidden).sample);
        assert!(!on(PanelPresence::Hidden).draw);
        assert!(on(PanelPresence::Unfocused).sample);
        assert!(!on(PanelPresence::Unfocused).draw);
        assert!(!on(PanelPresence::SidebarCollapsed).sample);
        assert!(!on(PanelPresence::FocusMode).sample);
        assert!(on(PanelPresence::Active).draw);
    }

    #[test]
    fn icmp_follows_the_latency_view_not_focus_or_the_background_switch() {
        assert!(icmp_probes(true, true));
        assert!(!icmp_probes(false, true));
        assert!(!icmp_probes(true, false));
        assert!(!icmp_probes(false, false));
        assert_ne!(SAMPLE_GAP, 0.0);
        assert!(SAMPLE_GAP < 0.0);
    }
}
