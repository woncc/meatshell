//! Lightweight poller for local machine stats (CPU / memory / network).
//!
//! `sysinfo` is already a dependency for many Rust desktop apps; it gives us
//! cross-platform data with ~2% CPU overhead at 1-second cadence.

use std::time::Duration;

use sysinfo::{Disks, Networks, System};

use super::system_types::{SystemSampler, SystemSnapshot};

impl SystemSampler {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let nets = Networks::new_with_refreshed_list();
        let last_rx_total = nets.iter().map(|(_, d)| d.total_received()).sum();
        let last_tx_total = nets.iter().map(|(_, d)| d.total_transmitted()).sum();
        let disks = Disks::new_with_refreshed_list();
        Self {
            sys,
            nets,
            disks,
            last_rx_total,
            last_tx_total,
            last_instant: std::time::Instant::now(),
        }
    }

    /// Recommended poll interval for a UI sidebar.
    pub fn recommended_interval() -> Duration {
        Duration::from_millis(1000)
    }

    pub fn sample(&mut self) -> SystemSnapshot {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.nets.refresh(true);

        let cpu_percent = self.sys.global_cpu_usage() / 100.0;

        let mem_total = self.sys.total_memory();
        let mem_used = self.sys.used_memory();
        let mem_percent = if mem_total > 0 {
            mem_used as f32 / mem_total as f32
        } else {
            0.0
        };

        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();
        let swap_percent = if swap_total > 0 {
            swap_used as f32 / swap_total as f32
        } else {
            0.0
        };

        // RX / TX bytes/sec from the delta across the iface list.
        let rx_total: u64 = self.nets.iter().map(|(_, d)| d.total_received()).sum();
        let tx_total: u64 = self.nets.iter().map(|(_, d)| d.total_transmitted()).sum();
        let now = std::time::Instant::now();
        let elapsed = now
            .duration_since(self.last_instant)
            .as_secs_f64()
            .max(0.001);
        let rx_delta = rx_total.saturating_sub(self.last_rx_total);
        let tx_delta = tx_total.saturating_sub(self.last_tx_total);
        self.last_rx_total = rx_total;
        self.last_tx_total = tx_total;
        self.last_instant = now;
        let net_rx_per_sec = (rx_delta as f64 / elapsed) as u64;
        let net_tx_per_sec = (tx_delta as f64 / elapsed) as u64;

        // Local filesystems (slow-changing, but cheap to refresh).
        self.disks.refresh(true);
        let disks: Vec<(String, u64, u64)> = self
            .disks
            .iter()
            .map(|d| {
                (
                    d.mount_point().to_string_lossy().to_string(),
                    d.available_space(),
                    d.total_space(),
                )
            })
            .filter(|(_, _, total)| *total > 0)
            .collect();

        SystemSnapshot {
            cpu_percent,
            mem_percent,
            swap_percent,
            mem_used_mib: mem_used / 1024 / 1024,
            mem_total_mib: mem_total / 1024 / 1024,
            swap_used_mib: swap_used / 1024 / 1024,
            swap_total_mib: swap_total / 1024 / 1024,
            net_bytes_per_sec: net_rx_per_sec + net_tx_per_sec,
            net_rx_per_sec,
            net_tx_per_sec,
            disks,
        }
    }
}

/// Format a 1-minute load average with exactly 3 significant digits.
///
/// `0.123`, `1.23`, and `12.3` are the usual shapes. Trailing zeros stay so
/// a value like `1.20` does not collapse to 2 digits. Zero is `0.000`.
/// Non-finite or negative input is `--`.
pub fn format_load_average(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return "--".to_string();
    }
    if value == 0.0 {
        return "0.000".to_string();
    }
    let mut exp = value.log10().floor() as i32;
    let mut coef = (value / 10f64.powi(exp) * 100.0).round();
    if coef >= 1000.0 {
        coef /= 10.0;
        exp += 1;
    }
    if coef <= 0.0 || !coef.is_finite() {
        return "0.000".to_string();
    }
    let digits = format!("{:03}", coef as i32);
    if exp < -6 || exp > 8 {
        let coeff = coef / 100.0;
        return format!("{coeff:.2}e{exp}");
    }
    if exp < 0 {
        let zeros = (-exp - 1) as usize;
        let mut out = String::from("0.");
        out.extend(std::iter::repeat('0').take(zeros));
        out.push_str(&digits);
        return out;
    }
    if exp >= 2 {
        let mut out = digits;
        out.extend(std::iter::repeat('0').take((exp - 2) as usize));
        return out;
    }
    let split = exp as usize + 1;
    let (whole, frac) = digits.split_at(split);
    format!("{whole}.{frac}")
}

/// Format a used/total memory pair (both in MiB) for the narrow sidebar.
/// Below 1 GiB it stays in megabytes (`512/2048M`); at or above, it switches to
/// gigabytes and drops the decimal for whole or large values to stay compact
/// (`1.5G/16G`, `120G/256G`).
pub fn format_mem(used_mib: u64, total_mib: u64) -> String {
    if total_mib < 1024 {
        return format!("{used_mib}/{total_mib}M");
    }
    // MiB → GiB, with a tidy width: integer when round or ≥100, else one decimal.
    fn gib(mib: u64) -> String {
        let g = mib as f64 / 1024.0;
        if g.fract() == 0.0 || g >= 100.0 {
            (g as u64).to_string()
        } else {
            format!("{g:.1}")
        }
    }
    format!("{}G/{}G", gib(used_mib), gib(total_mib))
}

/// Human-readable network throughput (e.g. `"1.2 MB/s"`).
pub fn format_bytes_per_sec(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B/s", "KB/s", "MB/s", "GB/s"];
    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", bytes, UNITS[idx])
    } else {
        format!("{:.1} {}", value, UNITS[idx])
    }
}

/// Fixed sparkline pitch (`Sparkline.bar-pitch` in `ui/widgets.slint`).
pub const SPARKLINE_BAR_PITCH_PX: f32 = 3.0;
/// Side-dock padding between the panel edge and the sparkline (10px margin
/// plus 10px layout padding, each side). The latency chart uses this chrome.
pub const SPARKLINE_EDGE_CHROME_PX: f32 = 40.0;
/// Rate-axis label column at UI scale 1 (`56px` in `ui/sidebar.slint`).
pub const RATE_AXIS_LABEL_PX: f32 = 56.0;
/// Gap between that column and the sparkline.
pub const RATE_AXIS_GAP_PX: f32 = 4.0;
/// Widest side panel the dock drag will store. Matches `dock_stacks::MAX_THICK`.
pub const SPARKLINE_MAX_SIDEBAR_PX: f32 = 2600.0;

/// How many fixed-pitch bars fit in `graph_width_px`.
pub const fn sparkline_bars_for_width(graph_width_px: f32) -> usize {
    if !graph_width_px.is_finite() || graph_width_px <= 0.0 {
        return 1;
    }
    let bars = (graph_width_px / SPARKLINE_BAR_PITCH_PX) as usize;
    if bars == 0 {
        1
    } else {
        bars
    }
}

/// Samples kept so the widest side panel fills at [`SPARKLINE_BAR_PITCH_PX`].
/// The latency sparkline has no axis column, so it is the wider graph.
pub const SPARKLINE_HISTORY_LEN: usize =
    sparkline_bars_for_width(SPARKLINE_MAX_SIDEBAR_PX - SPARKLINE_EDGE_CHROME_PX);

/// Compact Y-axis rate. Same 1024-based steps as [`format_bytes_per_sec`], but
/// at most 4 significant digits and a single unit letter `K`/`M`/`G` — no
/// `B/s` suffix. The result is at most 6 characters (`987.6M`, `1.234G`,
/// `1023K`, `0`) so the sidebar axis never ellipsizes.
pub fn format_axis_rate(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "0".to_string();
    }
    const STEP: u128 = 1024;
    const UNITS: [&str; 4] = ["", "K", "M", "G"];
    let bytes = bytes_per_sec as u128;
    let mut unit = 0usize;
    let mut base: u128 = 1;
    while unit < 3 && bytes >= base * STEP {
        base *= STEP;
        unit += 1;
    }
    if unit == 3 && bytes / base >= STEP {
        let shown = (bytes / base).min(9999);
        return format!("{shown}G");
    }
    let mut text = format_axis_coef(bytes, base);
    if text.parse::<u128>().ok().is_some_and(|n| n >= STEP) {
        if unit < 3 {
            unit += 1;
            text = "1".to_string();
        } else {
            let shown = text.parse::<u128>().unwrap_or(STEP).min(9999);
            return format!("{shown}G");
        }
    }
    let mut out = text;
    out.push_str(UNITS[unit]);
    out
}

/// `numer/denom` with at most 4 significant digits, trailing zeros stripped.
/// Coefficients in `[1000, 1024)` stay integers so the label stays within 6
/// characters (`1023K`, not `1023.6K`).
fn format_axis_coef(numer: u128, denom: u128) -> String {
    let int_part = numer / denom;
    let decimals: u32 = if int_part >= 1000 {
        0
    } else if int_part >= 100 {
        1
    } else if int_part >= 10 {
        2
    } else {
        3
    };
    if decimals == 0 {
        return int_part.to_string();
    }
    let scale = 10u128.pow(decimals);
    let rounded = (numer * scale + denom / 2) / denom;
    let whole = rounded / scale;
    if whole >= 1000 {
        return whole.to_string();
    }
    let frac = rounded % scale;
    if frac == 0 {
        return whole.to_string();
    }
    let mut frac_s = format!("{frac:0width$}", width = decimals as usize);
    while frac_s.ends_with('0') {
        frac_s.pop();
    }
    format!("{whole}.{frac_s}")
}

#[cfg(test)]
mod load_average_tests {
    use super::format_load_average;

    #[test]
    fn three_significant_digits_match_the_sidebar_examples() {
        assert_eq!(format_load_average(0.123), "0.123");
        assert_eq!(format_load_average(1.23), "1.23");
        assert_eq!(format_load_average(12.3), "12.3");
    }

    #[test]
    fn rounds_and_keeps_exactly_three_significant_digits() {
        assert_eq!(format_load_average(0.0), "0.000");
        assert_eq!(format_load_average(0.1234), "0.123");
        assert_eq!(format_load_average(1.234), "1.23");
        assert_eq!(format_load_average(1.2), "1.20");
        assert_eq!(format_load_average(12.34), "12.3");
        assert_eq!(format_load_average(9.996), "10.0");
        assert_eq!(format_load_average(99.96), "100");
        assert_eq!(format_load_average(0.09996), "0.100");
        assert_eq!(format_load_average(0.001234), "0.00123");
        assert_eq!(format_load_average(1234.0), "1230");
        assert_eq!(format_load_average(f64::NAN), "--");
        assert_eq!(format_load_average(-1.0), "--");
    }
}

#[cfg(test)]
mod axis_rate_tests {
    use super::{
        format_axis_rate, sparkline_bars_for_width, SPARKLINE_EDGE_CHROME_PX,
        SPARKLINE_HISTORY_LEN, SPARKLINE_MAX_SIDEBAR_PX,
    };

    fn numeric_digits(label: &str) -> usize {
        label.chars().filter(|c| c.is_ascii_digit()).count()
    }

    fn assert_axis_label(bytes: u64) {
        let label = format_axis_rate(bytes);
        assert!(label.len() <= 6, "{bytes} -> {label}");
        assert!(numeric_digits(&label) <= 4, "{bytes} -> {label}");
        assert!(
            label
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || matches!(c, 'K' | 'M' | 'G')),
            "{bytes} -> {label}"
        );
        assert!(!label.contains('B'), "{label}");
        assert!(!label.contains('/'), "{label}");
        let letters = label.chars().filter(|c| c.is_ascii_alphabetic()).count();
        assert!(letters <= 1, "{label}");
    }

    #[test]
    fn axis_labels_use_kmg_and_fit_six_characters() {
        assert_eq!(format_axis_rate(0), "0");
        assert_eq!(format_axis_rate(1024), "1K");
        assert_eq!(format_axis_rate(2048), "2K");
        // 4-digit coefficient plus a unit letter (same shape as `1234K`).
        assert_eq!(format_axis_rate(1023 * 1024), "1023K");
        assert_eq!(format_axis_rate(9876 * 1024 * 1024 / 10), "987.6M");
        assert_eq!(format_axis_rate(1234 * 1024 * 1024 * 1024 / 1000), "1.234G");
        let samples = [
            1u64,
            9,
            10,
            99,
            100,
            512,
            999,
            1000,
            1023,
            1024,
            12_340,
            98_760,
            999_950,
            1_234_000,
            987_600_000,
            1_234_000_000,
            999_950_000_000,
            u64::MAX,
        ];
        for bytes in samples {
            assert_axis_label(bytes);
        }
        for exp in 0..7 {
            let pow = 1024u64.saturating_pow(exp);
            for mant in [1u64, 2, 12, 123, 999, 1000, 1023] {
                assert_axis_label(mant.saturating_mul(pow));
            }
        }
        assert_eq!(format_axis_rate(u64::MAX), "9999G");
    }

    #[test]
    fn history_covers_the_widest_side_panel() {
        let latency_bars =
            sparkline_bars_for_width(SPARKLINE_MAX_SIDEBAR_PX - SPARKLINE_EDGE_CHROME_PX);
        assert_eq!(SPARKLINE_HISTORY_LEN, latency_bars);
        assert!(SPARKLINE_HISTORY_LEN > 160);
        assert!(latency_bars > sparkline_bars_for_width(520.0 - SPARKLINE_EDGE_CHROME_PX));
    }
}
