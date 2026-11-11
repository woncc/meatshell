// Entry point. Wires the Slint UI to the config store, system sampler and
// SSH session manager.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod allocator;

#[global_allocator]
static GLOBAL: allocator::Allocator = allocator::Allocator;

mod app;
mod config;
mod i18n;
mod layout;
mod logging;
mod rdp;
mod resource;
mod session;
mod session_test;
mod sftp;
mod ssh;
mod terminal;
mod tunnel;
mod ui;
mod wallpaper;
mod webdav;

const MCP_REMOVED: &str = "MCP has been removed from this build";
const CLI_REMOVED: &str = "CLI has been removed from this build";

enum StartMode {
    App,
    Version,
}

impl StartMode {
    fn detect(args: &[String]) -> Self {
        if args.iter().any(|arg| arg == "--version" || arg == "-V") {
            Self::Version
        } else {
            Self::App
        }
    }
}

/// Old `mcp` / `cli` invocations must not fall through into the desktop window.
/// `configure_profile` strips `--data-dir` first, so the subcommand is `args[1]`.
fn removed_frontend_message(args: &[String]) -> Option<&'static str> {
    match args.get(1).map(String::as_str) {
        Some("mcp") => Some(MCP_REMOVED),
        Some("cli") => Some(CLI_REMOVED),
        _ => None,
    }
}

fn main() -> anyhow::Result<()> {
    let result = run();
    if let Err(error) = &result {
        // In a Windows GUI build there is no console to retain main's error.
        // The file layer is synchronous, including with panic=abort.
        tracing::error!(error = %format!("{error:#}"), "application terminated with an error");
    }
    result
}

fn run() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
    config::configure_profile(&mut args)?;
    if let Some(message) = removed_frontend_message(&args) {
        eprintln!("{message}");
        std::process::exit(1);
    }
    if args.iter().any(|arg| arg == "--config-info") {
        let store = config::ConfigStore::load()?;
        println!(
            "{}",
            serde_json::json!({
                "executable": std::env::current_exe()?,
                "version": env!("CARGO_PKG_VERSION"),
                "data_dir": config::data_dir(),
                "session_count": store.sessions().len(),
            })
        );
        return Ok(());
    }

    let mode = StartMode::detect(&args);
    if matches!(mode, StartMode::Version) {
        println!("meatshell {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    init_tracing();
    install_panic_logging();

    // macOS defaults to Slint's CPU renderer. FemtoVG and Skia remain available
    // in Settings -> Interface -> Rendering for users who prefer GPU rendering.
    //
    // History: 0.4.10 force-set SLINT_BACKEND=winit-skia to work around femtovg's
    // CoreText font lookup failing on macOS 26 / Tahoe (all text vanished, #108).
    // That fix shipped without on-device verification and turned out to *break* a
    // different set of Macs (Apple Silicon M5 / 26.5): Skia couldn't resolve the
    // "PingFang SC" UI font and all text vanished there instead (#129). Icons
    // survived in both cases because Material Icons is an embedded font.
    //
    // Neither GPU renderer works for every macOS machine, so software rendering
    // is the compatibility default. Users can select FemtoVG or Skia under
    // Settings -> Interface -> Rendering. The
    // SLINT_BACKEND=winit-skia diagnostic override remains available and takes
    // precedence over the saved setting. The renderer-skia feature is compiled in
    // on macOS (see Cargo.toml), so switching does not require a rebuild.

    // ── IME policy ───────────────────────────────────────────────────────────
    // NOTE: We deliberately DO **NOT** call `ImmDisableIME` here.
    //
    // An earlier version disabled the IME for the whole Slint event-loop thread
    // to work around a vim `:q!` glitch (Chinese IMEs intercept letter keys and,
    // on a Shift press, discard the in-flight pinyin).  But disabling the IME
    // also makes 中文输入 completely impossible — there is no composition window
    // at all, which is exactly the "无法输入任何中文" bug.
    //
    // Chinese input now flows through the hidden `ime-input` TextInput in
    // terminal_view.slint: composition happens there, and committed text is
    // forwarded to the PTY via the `edited` callback.  The vim/Shift side-effects
    // are handled instead by the C0-marker + 3-layer Backspace filters in
    // `app::on_send_key`, so we no longer need (and must not use) ImmDisableIME.
    let intent = app::launch::parse(&args);
    app::run(intent)
}

fn install_panic_logging() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(
            panic = %info,
            backtrace = %std::backtrace::Backtrace::force_capture(),
            "application panicked"
        );
        previous(info);
    }));
}

/// Set up tracing: stderr (honours RUST_LOG, default info) **plus** a capped
/// `error.log` file at WARN and above so users can send diagnostics — e.g. a
/// bastion disconnect reason — without setting RUST_LOG (#86).
fn init_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    // Third-party noise routed through `log` → tracing: ICU4X data-error warnings
    // (icu_provider dependency) and fontdb's "malformed font" warning for fonts it
    // can't parse but harmlessly skips (e.g. Windows' mstmc.ttf). Silence on every
    // layer; keep fontdb at `error` so genuine failures still surface.
    fn quiet_noise(mut f: EnvFilter) -> EnvFilter {
        for d in [
            "icu_provider=off",
            "icu_segmenter=off",
            "icu_normalizer=off",
            "fontdb=error",
        ] {
            if let Ok(dir) = d.parse() {
                f = f.add_directive(dir);
            }
        }
        f
    }

    let env_filter =
        quiet_noise(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(env_filter);

    // One file, capped at 50 MiB, auto-overwriting when full (5 MiB was too
    // small to diagnose anything useful).
    let file_layer = logging::path()
        .and_then(|p| logging::CappedFile::open(p, 50 * 1024 * 1024).ok())
        .map(|cf| {
            fmt::layer()
                .with_ansi(false)
                .with_writer(logging::CappedWriter::new(cf))
                .with_filter(quiet_noise(EnvFilter::new("warn")))
        });

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_is_written_to_error_log() {
        const CHILD_PROFILE: &str = "MEATSHELL_PANIC_LOG_TEST_PROFILE";
        if let Some(profile) = std::env::var_os(CHILD_PROFILE) {
            let mut args = vec![
                "meatshell".to_owned(),
                "--data-dir".to_owned(),
                profile.to_string_lossy().into_owned(),
            ];
            config::configure_profile(&mut args).unwrap();
            init_tracing();
            install_panic_logging();
            panic!("panic-log-regression-marker");
        }

        // A child process isolates the global panic hook and tracing subscriber
        // from other tests, and exercises the actual synchronous file writer.
        let profile = std::env::temp_dir().join(format!(
            "meatshell-panic-log-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::panic_is_written_to_error_log", "--nocapture"])
            .env(CHILD_PROFILE, &profile)
            .output()
            .unwrap();
        let log = std::fs::read_to_string(profile.join("log/error.log")).unwrap();
        std::fs::remove_dir_all(&profile).unwrap();
        assert!(!output.status.success());
        assert!(log.contains("application panicked"));
        assert!(log.contains("panic-log-regression-marker"));
        assert!(log.contains("main.rs:"));
        assert!(log.contains("backtrace="));
    }


    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    #[test]
    fn removed_frontends_do_not_select_the_gui() {
        assert_eq!(
            removed_frontend_message(&argv(&["meatshell", "mcp", "serve"])),
            Some(MCP_REMOVED)
        );
        assert_eq!(
            removed_frontend_message(&argv(&["meatshell", "mcp"])),
            Some(MCP_REMOVED)
        );
        assert_eq!(
            removed_frontend_message(&argv(&[
                "meatshell",
                "mcp",
                "serve",
                "--http-config",
                "http.json",
            ])),
            Some(MCP_REMOVED)
        );
        assert_eq!(
            removed_frontend_message(&argv(&["meatshell", "cli"])),
            Some(CLI_REMOVED)
        );
        assert_eq!(
            removed_frontend_message(&argv(&["meatshell", "cli", "sessions"])),
            Some(CLI_REMOVED)
        );
        assert_eq!(
            removed_frontend_message(&argv(&["meatshell", "cli", "exec", "target"])),
            Some(CLI_REMOVED)
        );
        assert!(removed_frontend_message(&argv(&["meatshell"])).is_none());
        assert!(removed_frontend_message(&argv(&["meatshell", "--config-info"])).is_none());
        assert!(removed_frontend_message(&argv(&["meatshell", "--version"])).is_none());
        assert!(matches!(
            StartMode::detect(&argv(&["meatshell"])),
            StartMode::App
        ));
        assert!(matches!(
            StartMode::detect(&argv(&["meatshell", "--version"])),
            StartMode::Version
        ));
        assert!(matches!(
            StartMode::detect(&argv(&["meatshell", "-V"])),
            StartMode::Version
        ));
    }
}
