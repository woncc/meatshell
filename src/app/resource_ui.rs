use super::*;

pub(super) fn push_ring(buf: &mut Vec<f32>, val: f32) {
    if buf.len() != NET_HISTORY_LEN {
        *buf = vec![0.0; NET_HISTORY_LEN];
    }
    buf.remove(0);
    buf.push(val);
}

pub(super) fn push_rate(hist: &mut RateHist, rx: f32, tx: f32) {
    push_ring(&mut hist.rx, rx);
    push_ring(&mut hist.tx, tx);
}

/// How many fixed-pitch bars fit in a sparkline of `graph_width_px`.
/// Pitch is 3px, matching `Sparkline.bar-pitch` in `ui/widgets.slint`.
pub(super) fn net_bars_for_graph_width(graph_width_px: f32) -> usize {
    crate::resource::system::sparkline_bars_for_width(graph_width_px)
}

/// Newest `bars` samples. The sparkline draws only this suffix at a fixed
/// pitch, so a spike older than the current sidebar width is not visible.
pub(super) fn visible_tail(buf: &[f32], bars: usize) -> &[f32] {
    let n = bars.max(1);
    if buf.len() <= n {
        buf
    } else {
        &buf[buf.len() - n..]
    }
}

/// Auto-scale a raw history to 0..1 against its own peak so the sparkline
/// uses the full height (like FinalShell's relative graph).
pub(super) fn normalized_values(buf: &[f32]) -> Vec<f32> {
    let max = buf.iter().cloned().fold(1.0_f32, f32::max);
    buf.iter().map(|v| (v / max).clamp(0.0, 1.0)).collect()
}

pub(super) fn normalized_model(buf: &[f32]) -> ModelRc<f32> {
    ModelRc::from(Rc::new(VecModel::from(normalized_values(buf))))
}

/// Latency bars for the samples the current plot width can draw.
pub(super) fn normalized_visible(buf: &[f32], bars: usize) -> ModelRc<f32> {
    normalized_model(visible_tail(buf, bars))
}

/// Download and upload scaled against one shared peak, plus the Y-axis labels
/// for that peak. Callers pass the visible window, not the whole ring: a
/// spike that has scrolled off the left must not set the axis. Idle history
/// labels `0` rather than a fake throughput.
pub(super) struct ScaledRates {
    pub(super) rx: Vec<f32>,
    pub(super) tx: Vec<f32>,
    pub(super) axis_top: String,
    pub(super) axis_mid: String,
}

pub(super) fn scale_rate_pair(rx: &[f32], tx: &[f32]) -> ScaledRates {
    let peak = rx
        .iter()
        .chain(tx.iter())
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0_f32, f32::max)
        .max(0.0);
    let denom = peak.max(1.0);
    let scale = |v: f32| {
        if v.is_finite() {
            (v / denom).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let (axis_top, axis_mid) = if peak <= 0.0 {
        ("0".to_string(), "0".to_string())
    } else {
        (
            format_axis_rate(peak.round() as u64),
            format_axis_rate((peak / 2.0).round() as u64),
        )
    };
    ScaledRates {
        rx: rx.iter().copied().map(scale).collect(),
        tx: tx.iter().copied().map(scale).collect(),
        axis_top,
        axis_mid,
    }
}

/// Scale only the samples the sparkline will draw for `bars` slots.
pub(super) fn scale_visible_rates(rx: &[f32], tx: &[f32], bars: usize) -> ScaledRates {
    scale_rate_pair(visible_tail(rx, bars), visible_tail(tx, bars))
}

pub(super) fn float_model(values: &[f32]) -> ModelRc<f32> {
    ModelRc::from(Rc::new(VecModel::from(values.to_vec())))
}

/// Build the filesystem-usage model (path, "avail/total", used fraction).
pub(super) fn disk_rows(disks: &[(String, u64, u64)]) -> Vec<DiskInfo> {
    disks
        .iter()
        .map(|(mount, avail, total)| {
            let used = total.saturating_sub(*avail);
            let percent = if *total > 0 {
                used as f32 / *total as f32
            } else {
                0.0
            };
            DiskInfo {
                path: mount.clone().into(),
                detail: format!("{}/{}", format_size(*avail), format_size(*total)).into(),
                percent,
            }
        })
        .collect()
}

pub(super) fn disk_model(disks: &[(String, u64, u64)]) -> ModelRc<DiskInfo> {
    ModelRc::from(Rc::new(VecModel::from(disk_rows(disks))))
}

/// Build the process-monitor model for the popup (#23). `cpu`/`mem` are
/// pre-formatted to one decimal; `mem_size` is resident size via [`format_size`];
/// `cpu_frac` (0..1) drives the row's load bar.
pub(super) fn set_process_action_error(weak: &slint::Weak<ProcWindow>, message: &str) {
    if let Some(window) = weak.upgrade() {
        window.set_action_busy(false);
        window.set_action_error(true);
        window.set_action_status(message.into());
    }
}

/// A root login can signal any process directly. Non-root logins may signal
/// only their own processes; root and other users' processes require `su`.
pub(super) fn process_needs_root(current_user: &str, process_user: &str) -> bool {
    current_user != "root" && process_user != current_user
}

/// Column indexes match the process-window headers: 0 PID, 1 user, 2 CPU%,
/// 3 MEM%, 4 resident size, 5 command. Anything else sorts by CPU.
pub(super) fn proc_rows(
    procs: &[ProcInfo],
    current_user: &str,
    tab_id: &str,
    sort_col: i32,
    sort_desc: bool,
) -> Vec<ProcRow> {
    let mut order: Vec<&ProcInfo> = procs.iter().collect();
    order.sort_by(|a, b| {
        let ord = match sort_col {
            0 => a.pid.cmp(&b.pid),
            1 => cmp_ignore_case(&a.user, &b.user),
            3 => a.mem.total_cmp(&b.mem),
            4 => a.rss_kib.cmp(&b.rss_kib),
            5 => cmp_ignore_case(&a.command, &b.command),
            _ => a.cpu.total_cmp(&b.cpu),
        };
        let ord = if sort_desc { ord.reverse() } else { ord };
        ord.then(a.pid.cmp(&b.pid))
    });
    order
        .into_iter()
        .map(|p| ProcRow {
            tab_id: tab_id.into(),
            pid: p.pid.to_string().into(),
            user: p.user.clone().into(),
            cpu: format!("{:.1}", p.cpu).into(),
            mem: format!("{:.1}", p.mem).into(),
            mem_size: format_size(p.rss_kib.saturating_mul(1024)).into(),
            command: p.command.clone().into(),
            cpu_frac: (p.cpu / 100.0).clamp(0.0, 1.0),
            own_process: !process_needs_root(current_user, &p.user),
        })
        .collect()
}

fn cmp_ignore_case(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

pub(super) fn proc_sort(win: &AppWindow) -> (i32, bool) {
    (
        win.get_proc_sort_col().clamp(0, 5),
        win.get_proc_sort_desc(),
    )
}

#[cfg(test)]
mod net_history_tests {
    use super::super::dock_stacks::MAX_THICK;
    use super::super::NET_HISTORY_LEN;
    use super::net_bars_for_graph_width;
    use crate::resource::system::{
        sparkline_bars_for_width, RATE_AXIS_GAP_PX, RATE_AXIS_LABEL_PX, SPARKLINE_EDGE_CHROME_PX,
        SPARKLINE_MAX_SIDEBAR_PX,
    };

    #[test]
    fn sidebar_width_changes_time_range_not_bar_pitch() {
        // Edge padding is 40px. The rate axis adds its label column and gap;
        // the latency sparkline does not, so it is the wider graph.
        let rate_chrome = SPARKLINE_EDGE_CHROME_PX + RATE_AXIS_LABEL_PX + RATE_AXIS_GAP_PX;
        let bars = |sidebar_px: f32| net_bars_for_graph_width(sidebar_px - rate_chrome);
        let latency_bars =
            |sidebar_px: f32| sparkline_bars_for_width(sidebar_px - SPARKLINE_EDGE_CHROME_PX);
        assert_eq!(bars(220.0), 40);
        assert_eq!(bars(160.0), 20);
        assert_eq!(bars(520.0), 140);
        assert!(bars(160.0) < bars(220.0));
        assert!(bars(520.0) > bars(220.0));
        assert_eq!(MAX_THICK, SPARKLINE_MAX_SIDEBAR_PX);
        // Fixed pitch: the ring covers the current maximum, including the
        // latency chart that has no axis column. Shrinking still keeps those
        // samples; the old 520px/160 cap does not.
        assert!(NET_HISTORY_LEN >= bars(MAX_THICK));
        assert_eq!(NET_HISTORY_LEN, latency_bars(MAX_THICK));
        assert!(NET_HISTORY_LEN > latency_bars(520.0));
        assert!(NET_HISTORY_LEN > 160);
        assert_eq!(net_bars_for_graph_width(0.0), 1);
        assert_eq!(net_bars_for_graph_width(f32::NAN), 1);
    }

    #[test]
    fn up_and_down_share_one_axis() {
        let scaled = super::scale_rate_pair(&[0.0, 1024.0], &[0.0, 2048.0]);
        assert_eq!(scaled.axis_top, "2K");
        assert_eq!(scaled.axis_mid, "1K");
        assert!(scaled.axis_top.len() <= 6);
        assert!(scaled.axis_mid.len() <= 6);
        assert!((scaled.tx.last().copied().unwrap() - 1.0).abs() < 0.001);
        assert!((scaled.rx.last().copied().unwrap() - 0.5).abs() < 0.001);
        let idle = super::scale_rate_pair(&[0.0, f32::NAN], &[0.0]);
        assert_eq!(idle.axis_top, "0");
        assert_eq!(idle.axis_mid, "0");
        assert_eq!(idle.rx, vec![0.0, 0.0]);
    }

    /// A spike that has scrolled off a narrow sidebar must not keep the axis
    /// (or the visible bar heights) at that old peak. Widening includes it again.
    #[test]
    fn axis_follows_the_visible_window_not_the_ring() {
        let spike = 8.0 * 1024.0 * 1024.0 * 1024.0;
        let mut rx = vec![0.0; 20];
        rx[0] = spike;
        rx[18] = 175.0;
        rx[19] = 50.0;
        let tx = vec![245.0; 20];
        let narrow = super::scale_visible_rates(&rx, &tx, 4);
        assert_eq!(narrow.rx.len(), 4);
        assert_eq!(narrow.tx.len(), 4);
        assert_eq!(narrow.axis_top, "245");
        assert!(!narrow.axis_top.contains('G'), "{}", narrow.axis_top);
        assert!((narrow.tx[0] - 1.0).abs() < 0.001);
        assert!((narrow.rx[2] - (175.0 / 245.0)).abs() < 0.001);
        let wide = super::scale_visible_rates(&rx, &tx, 20);
        assert_eq!(wide.rx.len(), 20);
        assert_eq!(wide.axis_top, "8G");
        assert!((wide.rx[19] - (50.0 / spike)).abs() < 0.001);

        let mut latency = vec![2.0; 40];
        latency[0] = 800.0;
        latency[39] = 20.0;
        let drawn = super::normalized_values(super::visible_tail(&latency, 5));
        assert_eq!(drawn.len(), 5);
        assert!((drawn[4] - 1.0).abs() < 0.001, "{drawn:?}");
        let full = super::normalized_values(super::visible_tail(&latency, 40));
        assert!((full[39] - (20.0 / 800.0)).abs() < 0.001);
    }

    /// Speed and latency each show one reading plus one 48px sparkline, and
    /// neither row stretches. A stretching latency body is what opened the
    /// blank band between the value and the orange chart.
    #[test]
    fn latency_mode_does_not_reserve_unused_vertical_space() {
        let src = include_str!("../../ui/sidebar.slint");
        let start = src
            .find("if root.latency-mode : VerticalLayout {")
            .expect("latency layout");
        let rest = &src[start..];
        let end = rest.find("component DiskBlock").expect("disk block");
        let block = &rest[..end];
        assert!(
            block.contains("vertical-stretch: 0"),
            "latency body must stay at its preferred height"
        );
        assert!(
            block.contains("alignment: start"),
            "extra height must not separate the latency value from its sparkline"
        );
        assert_eq!(block.matches("Sparkline {").count(), 1);
        assert!(block.contains("height: 48px"));
        assert!(
            !block.contains("NetGraph {"),
            "latency mode must not keep a second rate graph"
        );

        let speed = local_metric_slots(false);
        let latency = local_metric_slots(true);
        assert_eq!(speed.len(), latency.len(), "no extra latency slot");
        assert!(speed
            .iter()
            .chain(latency.iter())
            .all(|row| row.stretch == 0.0));
        assert_eq!(phantom_gap(&speed, 480.0), 0.0);
        assert_eq!(phantom_gap(&latency, 480.0), 0.0);
        let speed_h = preferred_height(&speed);
        let latency_h = preferred_height(&latency);
        assert!(
            (speed_h - latency_h).abs() < 24.0,
            "speed {speed_h}px vs latency {latency_h}px"
        );
    }

    struct MetricSlot {
        stretch: f32,
        preferred_px: f32,
    }

    fn local_metric_slots(latency_mode: bool) -> [MetricSlot; 2] {
        if latency_mode {
            [
                MetricSlot {
                    stretch: 0.0,
                    preferred_px: 14.0,
                },
                MetricSlot {
                    stretch: 0.0,
                    preferred_px: 48.0,
                },
            ]
        } else {
            [
                MetricSlot {
                    stretch: 0.0,
                    preferred_px: 16.0,
                },
                MetricSlot {
                    stretch: 0.0,
                    preferred_px: 48.0,
                },
            ]
        }
    }

    fn preferred_height(rows: &[MetricSlot]) -> f32 {
        let spacing = 3.0 * rows.len().saturating_sub(1) as f32;
        spacing + rows.iter().map(|row| row.preferred_px).sum::<f32>()
    }

    fn phantom_gap(rows: &[MetricSlot], sidebar_extra_px: f32) -> f32 {
        let stretch: f32 = rows.iter().map(|row| row.stretch).sum();
        if stretch > 0.0 {
            sidebar_extra_px
        } else {
            0.0
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/process_monitor/mod.rs"]
mod process_row_tests;

pub(super) fn metric_rows(
    cpu: f32,
    mem: f32,
    swap: f32,
    mem_detail: impl Into<SharedString>,
    swap_detail: impl Into<SharedString>,
) -> Vec<SysMetricRow> {
    vec![
        SysMetricRow {
            label: "CPU".into(),
            percent: cpu,
            detail: "".into(),
            kind: 0,
        },
        SysMetricRow {
            label: t("内存", "Memory").into(),
            percent: mem,
            detail: mem_detail.into(),
            kind: 1,
        },
        SysMetricRow {
            label: t("交换", "Swap").into(),
            percent: swap,
            detail: swap_detail.into(),
            kind: 2,
        },
    ]
}

pub(super) fn net_rows(net: &[(String, u64, u64)]) -> Vec<SysNetRow> {
    net.iter()
        .map(|(name, rx, tx)| SysNetRow {
            name: name.clone().into(),
            up: format_bytes_per_sec(*tx).into(),
            down: format_bytes_per_sec(*rx).into(),
        })
        .collect()
}

pub(super) fn pairs_to_overview_rows(pairs: &[(String, String)]) -> Vec<SysInfoRow> {
    pairs
        .chunks(2)
        .map(|chunk| {
            let first = &chunk[0];
            let second = chunk.get(1);
            SysInfoRow {
                c1: first.0.clone().into(),
                c2: first.1.clone().into(),
                c3: second.map(|p| p.0.clone()).unwrap_or_default().into(),
                c4: second.map(|p| p.1.clone()).unwrap_or_default().into(),
                c5: "".into(),
            }
        })
        .collect()
}

pub(super) fn pairs_to_one_row(pairs: &[(String, String)]) -> Vec<SysInfoRow> {
    let value = |idx: usize| {
        pairs
            .get(idx)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "-".to_string())
    };
    vec![SysInfoRow {
        c1: value(0).into(),
        c2: value(1).into(),
        c3: value(2).into(),
        c4: value(3).into(),
        c5: value(4).into(),
    }]
}

pub(super) fn pairs_to_rows(pairs: &[(String, String)], width: usize) -> Vec<SysInfoRow> {
    pairs
        .chunks(width)
        .filter(|chunk| {
            chunk
                .iter()
                .any(|(_, v)| !v.trim().is_empty() && v.trim() != "-")
        })
        .map(|chunk| {
            let value = |idx: usize| {
                chunk
                    .get(idx)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| "-".to_string())
            };
            SysInfoRow {
                c1: value(0).into(),
                c2: value(1).into(),
                c3: value(2).into(),
                c4: value(3).into(),
                c5: value(4).into(),
            }
        })
        .collect()
}

pub(super) fn cpu_usage_detail_rows(pairs: &[(String, String)]) -> Vec<SysInfoRow> {
    let value = |idx: usize| {
        pairs
            .get(idx)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "0.0%".to_string())
    };
    let extra = pairs
        .iter()
        .skip(4)
        .map(|(k, v)| format!("{k} {v}"))
        .collect::<Vec<_>>()
        .join(" / ");
    vec![SysInfoRow {
        c1: value(0).into(),
        c2: value(2).into(),
        c3: value(1).into(),
        c4: value(3).into(),
        c5: extra.into(),
    }]
}

pub(super) fn tuple5_rows(rows: &[(String, String, String, String, String)]) -> Vec<SysInfoRow> {
    rows.iter()
        .map(|r| SysInfoRow {
            c1: r.0.clone().into(),
            c2: r.1.clone().into(),
            c3: r.2.clone().into(),
            c4: r.3.clone().into(),
            c5: r.4.clone().into(),
        })
        .collect()
}

/// Mirror the main window's theme/scale/UI-font onto the detached process
/// window. Theme is a per-window Slint global, so a detached window keeps its
/// compile-time (dark) defaults until we copy these across (#23).
pub(super) fn sync_proc_theme(main: &AppWindow, proc: &ProcWindow) {
    proc.set_dark_mode(main.get_dark_mode());
    proc.set_ui_scale(main.get_ui_scale());
    proc.set_ui_font_family(main.get_ui_font_family());
    // Mirror the immersive wallpaper so the detached window shares the frosted
    // backdrop instead of a flat panel.
    proc.set_wallpaper_img(main.get_wallpaper_img());
    proc.set_wallpaper_active(main.get_wallpaper_active());
    proc.set_wp_accent(main.get_wp_accent());
    proc.set_wp_tint(main.get_wp_tint());
}

pub(super) fn sync_system_info_theme(main: &AppWindow, sys: &SystemInfoWindow) {
    sys.set_dark_mode(main.get_dark_mode());
    sys.set_ui_scale(main.get_ui_scale());
    sys.set_ui_font_family(main.get_ui_font_family());
    sys.set_wallpaper_img(main.get_wallpaper_img());
    sys.set_wallpaper_active(main.get_wallpaper_active());
    sys.set_wp_accent(main.get_wp_accent());
    sys.set_wp_tint(main.get_wp_tint());
}

pub(super) fn sync_editor_theme(main: &AppWindow, editor: &EditorWindow) {
    editor.set_dark_mode(main.get_dark_mode());
    editor.set_ui_scale(main.get_ui_scale());
    editor.set_ui_font_family(main.get_ui_font_family());
    editor.set_wallpaper_img(main.get_wallpaper_img());
    editor.set_wallpaper_active(main.get_wallpaper_active());
    editor.set_wp_accent(main.get_wp_accent());
    editor.set_wp_tint(main.get_wp_tint());
}

pub(super) fn place_system_info_window(main: &AppWindow, sys: &SystemInfoWindow) {
    let Some((mon_x, mon_y, mon_w, mon_h)) = main
        .window()
        .with_winit_window(|ww| {
            let scale = ww.scale_factor().max(0.01);
            let monitor = ww.current_monitor().or_else(|| ww.primary_monitor())?;
            let pos = monitor.position();
            let size = monitor.size();
            Some((
                pos.x as f64 / scale,
                pos.y as f64 / scale,
                size.width as f64 / scale,
                size.height as f64 / scale,
            ))
        })
        .flatten()
    else {
        return;
    };

    let target_w = (mon_w * 0.5).clamp(760.0, (mon_w - 24.0).max(760.0));
    let target_h = (mon_h * 0.5).clamp(520.0, (mon_h - 24.0).max(520.0));
    let x = mon_x + (mon_w - target_w).max(0.0) / 2.0;
    let y = mon_y + (mon_h - target_h).max(0.0) / 2.0;

    // Use the Slint window API instead of the winit handle: hidden windows are
    // no longer materialized eagerly (they map on Wayland and pollute the
    // taskbar — see the vendor patch in i-slint-backend-winit), so the first
    // open may run before the native window exists. In that state the Slint
    // API stores the size/position on the adapter and applies it at creation;
    // with a live window it behaves like the winit calls did.
    sys.window()
        .set_size(slint::LogicalSize::new(target_w as f32, target_h as f32));
    sys.window()
        .set_position(slint::LogicalPosition::new(x as f32, y as f32));
}

/// Center the process monitor on the same physical monitor as the main window.
/// Physical coordinates avoid logical/physical rounding errors when the two
/// displays use different DPI scale factors. Keep the user's current process
/// window size; opening it should reposition, not reset a manual resize.
pub(super) fn place_process_window(main: &AppWindow, process: &ProcWindow) {
    let monitor = main
        .window()
        .with_winit_window(|ww| ww.current_monitor().or_else(|| ww.primary_monitor()))
        .flatten();
    let Some(monitor) = monitor else { return };
    let origin = monitor.position();
    let monitor_size = monitor.size();

    // The winit window may not exist yet on the first open (deferred creation,
    // see place_system_info_window); fall back to a zero size, which anchors
    // the window's top-left at the monitor center until the next open.
    // Wayland ignores client-side positioning entirely, so this only affects
    // X11/Windows first-open placement.
    let window_size = process
        .window()
        .with_winit_window(|ww| ww.outer_size())
        .unwrap_or_default();
    let x = origin.x + monitor_size.width.saturating_sub(window_size.width) as i32 / 2;
    let y = origin.y + monitor_size.height.saturating_sub(window_size.height) as i32 / 2;
    process
        .window()
        .set_position(slint::PhysicalPosition::new(x, y));
}
