use super::*;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Key, PointerEventButton, WindowEvent};

struct Backend(Rc<RefCell<Vec<Rc<MinimalSoftwareWindow>>>>);
impl slint::platform::Platform for Backend {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        self.0.borrow_mut().push(window.clone());
        Ok(window)
    }
}

fn settle(window: &MinimalSoftwareWindow) {
    for _ in 0..2 {
        slint::platform::update_timers_and_animations();
        let mut pixels = vec![slint::Rgb8Pixel::default(); 1000 * 850];
        window.request_redraw();
        window.draw_if_needed(|r| {
            r.render(&mut pixels, 1000);
        });
    }
    slint::platform::update_timers_and_animations();
}

fn key(window: &MinimalSoftwareWindow, text: impl Into<SharedString>) {
    let text = text.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}

fn host() -> (
    AppWindow,
    Rc<MinimalSoftwareWindow>,
    Rc<RefCell<Vec<Rc<MinimalSoftwareWindow>>>>,
) {
    let windows = Rc::new(RefCell::new(Vec::new()));
    slint::platform::set_platform(Box::new(Backend(windows.clone()))).unwrap();
    let ui = AppWindow::new().unwrap();
    ui.show().unwrap();
    let window = windows.borrow()[0].clone();
    window.set_size(slint::PhysicalSize::new(1000, 850));
    settle(&window);
    (ui, window, windows)
}

#[test]
fn windows_session_groups_render_without_layout_recursion() {
    std::thread::spawn(|| {
        use slint::private_unstable_api::re_exports::{LogicalLength, Orientation, VRc, WindowInner};

        let windows = Rc::new(RefCell::new(Vec::new()));
        slint::platform::set_platform(Box::new(Backend(windows.clone()))).unwrap();
        let ui = AppWindow::new().unwrap();
        ui.set_is_mac(false);
        ui.set_is_windows(true);
        ui.set_custom_titlebar(true);
        // A fresh Windows profile still has built-in local-shell rows.
        ui.set_sessions(Rc::new(VecModel::from(vec![SessionInfo {
            id: "local-powershell".into(),
            name: "PowerShell".into(),
            group: "system".into(),
            group_header: "system".into(),
            builtin: true,
            ..Default::default()
        }, SessionInfo {
            id: "saved-example".into(),
            name: "Example".into(),
            host: "example.invalid".into(),
            group: "servers".into(),
            group_header: "servers".into(),
            ..Default::default()
        }])).into());
        let tabs: ModelRc<TabInfo> = Rc::new(VecModel::from(vec![TabInfo {
            id: "welcome".into(),
            title: "New tab".into(),
            title_raw: "New tab".into(),
            title_len: 7,
            kind: "welcome".into(),
            ..Default::default()
        }])).into();
        ui.set_tabs(tabs.clone());
        ui.set_active_tab_id("welcome".into());
        ui.set_panes(Rc::new(VecModel::from(vec![PaneInfo {
            id: 1,
            w: 1200.0,
            h: 800.0,
            active_id: "welcome".into(),
            focused: true,
            tabs,
            ..Default::default()
        }])).into());

        // Winit queries horizontal constraints and sets the initial width before
        // querying vertical constraints. MinimalSoftwareWindow alone skips this
        // native startup path, so merely drawing a frame does not cover it.
        let runtime = WindowInner::from_pub(ui.window());
        let component_rc = runtime.component();
        let component = VRc::borrow_pin(&component_rc);
        let horizontal = component.as_ref().layout_info(Orientation::Horizontal);
        runtime.window_item().unwrap().as_pin_ref().width.set(LogicalLength::new(horizontal.preferred_bounded()));
        let vertical = component.as_ref().layout_info(Orientation::Vertical);
        assert!(horizontal.preferred_bounded() >= 720.0);
        assert!(vertical.preferred_bounded() >= 420.0);
        ui.show().unwrap();
        let window = windows.borrow()[0].clone();
        window.set_size(slint::PhysicalSize::new(1000, 850));
        settle(&window);
    })
    .join()
    .unwrap();
}

fn bind_cancellations(ui: &AppWindow, calls: Rc<RefCell<Vec<&'static str>>>, window_id: u64) {
    let weak = ui.as_weak();
    let count = calls.clone();
    ui.on_session_dialog_cancel(move || {
        count.borrow_mut().push("session");
        weak.upgrade().unwrap().set_dialog_open(false);
    });
    let weak = ui.as_weak();
    let count = calls.clone();
    ui.on_cred_reject(move || {
        count.borrow_mut().push("credentials");
        resolve_front_cred(&weak.upgrade().unwrap(), window_id, false);
    });
    let weak = ui.as_weak();
    let count = calls.clone();
    ui.on_mfa_cancel(move || {
        count.borrow_mut().push("mfa");
        resolve_front_mfa(&weak.upgrade().unwrap(), window_id, false);
    });
    let weak = ui.as_weak();
    let count = calls.clone();
    ui.on_hostkey_reject(move || {
        count.borrow_mut().push("host-key");
        resolve_front_hostkey(&weak.upgrade().unwrap(), window_id, false);
    });
    ui.on_paste_confirm_cancelled(move || {
        calls.borrow_mut().push("paste");
    });
}

#[test]
fn every_app_modal_cancels_after_keyboard_navigation() {
    std::thread::spawn(|| {
        let (ui, window, _) = host();
        let calls = Rc::new(RefCell::new(Vec::new()));
        bind_cancellations(&ui, calls.clone(), 10);
        type Case = (&'static str, fn(&AppWindow, bool), fn(&AppWindow) -> bool);
        let cases: &[Case] = &[
            (
                "shortcuts",
                AppWindow::set_shortcuts_open,
                AppWindow::get_shortcuts_open,
            ),
            (
                "about",
                AppWindow::set_about_open,
                AppWindow::get_about_open,
            ),
            (
                "settings",
                AppWindow::set_interface_open,
                AppWindow::get_interface_open,
            ),
            (
                "session",
                AppWindow::set_dialog_open,
                AppWindow::get_dialog_open,
            ),
            (
                "group",
                AppWindow::set_group_dialog_open,
                AppWindow::get_group_dialog_open,
            ),
            (
                "rename",
                AppWindow::set_tab_rename_open,
                AppWindow::get_tab_rename_open,
            ),
            (
                "commands",
                AppWindow::set_quick_cmd_manage_open,
                AppWindow::get_quick_cmd_manage_open,
            ),
            (
                "command-group",
                AppWindow::set_qcg_open,
                AppWindow::get_qcg_open,
            ),
            (
                "sftp",
                AppWindow::set_sftp_prompt_open,
                AppWindow::get_sftp_prompt_open,
            ),
            (
                "permissions",
                AppWindow::set_chmod_open,
                AppWindow::get_chmod_open,
            ),
            (
                "import",
                AppWindow::set_batch_import_open,
                AppWindow::get_batch_import_open,
            ),
            (
                "delete",
                AppWindow::set_confirm_delete_open,
                AppWindow::get_confirm_delete_open,
            ),
            (
                "paste",
                AppWindow::set_paste_confirm_open,
                AppWindow::get_paste_confirm_open,
            ),
            (
                "host-key",
                AppWindow::set_hostkey_prompt_open,
                AppWindow::get_hostkey_prompt_open,
            ),
            (
                "credentials",
                AppWindow::set_cred_prompt_open,
                AppWindow::get_cred_prompt_open,
            ),
            (
                "mfa",
                AppWindow::set_mfa_prompt_open,
                AppWindow::get_mfa_prompt_open,
            ),
            (
                "exit",
                AppWindow::set_confirm_close_open,
                AppWindow::get_confirm_close_open,
            ),
        ];
        for (name, open, is_open) in cases {
            open(&ui, true);
            settle(&window);
            assert!(ui.get_modal_open(), "{name} must own the modal layer");
            for _ in 0..32 {
                key(&window, Key::Tab);
            }
            key(&window, Key::Escape);
            settle(&window);
            assert!(
                !is_open(&ui),
                "{name}: Escape was lost after Tab navigation"
            );
            assert!(!ui.get_modal_open());
            ui.set_is_mac(true);
            open(&ui, true);
            settle(&window);
            for _ in 0..32 {
                key(&window, Key::Backtab);
            }
            let command = if cfg!(target_os = "macos") {
                Key::Meta
            } else {
                Key::Control
            };
            window.dispatch_event(WindowEvent::KeyPressed {
                text: command.into(),
            });
            key(&window, ".");
            window.dispatch_event(WindowEvent::KeyReleased {
                text: command.into(),
            });
            ui.set_is_mac(false);
            settle(&window);
            assert!(
                !is_open(&ui),
                "{name}: reverse navigation escaped the modal"
            );
        }
        for expected in ["session", "host-key", "credentials", "mfa", "paste"] {
            assert_eq!(calls.borrow().iter().filter(|&&s| s == expected).count(), 2);
        }
    })
    .join()
    .unwrap();
}

#[test]
fn top_layer_keeps_underlying_draft_and_uses_business_cancellation() {
    std::thread::spawn(|| {
        let (ui, window, _) = host();
        let calls = Rc::new(RefCell::new(Vec::new()));
        bind_cancellations(&ui, calls.clone(), 20);
        ui.set_dialog_open(true);
        ui.set_dialog_password("unsaved-fixture".into());
        settle(&window);
        ui.set_cred_password("typed-fixture".into());
        ui.set_cred_prompt_open(true);
        settle(&window);
        key(&window, Key::Escape);
        settle(&window);
        assert!(!ui.get_cred_prompt_open());
        assert!(ui.get_cred_password().is_empty());
        assert!(ui.get_dialog_open());
        assert_eq!(ui.get_dialog_password(), "unsaved-fixture");
        assert_eq!(&*calls.borrow(), &["credentials"]);
        ui.set_confirm_close_open(true);
        settle(&window);
        key(&window, Key::Escape);
        settle(&window);
        assert!(!ui.get_confirm_close_open());
        assert!(ui.get_dialog_open());
        key(&window, Key::Escape);
        settle(&window);
        assert!(!ui.get_dialog_open());
        assert!(ui.get_dialog_password().is_empty());
        assert_eq!(&*calls.borrow(), &["credentials", "session"]);
    })
    .join()
    .unwrap();
}

#[test]
fn confirmation_and_backdrop_policies_are_explicit() {
    std::thread::spawn(|| {
        let (ui, window, _) = host();
        let confirms = Rc::new(Cell::new(0));
        let count = confirms.clone();
        ui.on_paste_confirmed(move |_| count.set(count.get() + 1));
        ui.set_confirm_delete_open(true);
        settle(&window);
        key(&window, Key::Return);
        assert!(
            ui.get_confirm_delete_open(),
            "destructive Enter must remain opt-in"
        );
        key(&window, Key::Escape);
        settle(&window);
        ui.set_paste_confirm_open(true);
        settle(&window);
        let pos = slint::LogicalPosition::new(8., 8.);
        window.dispatch_event(WindowEvent::PointerPressed {
            position: pos,
            button: PointerEventButton::Left,
        });
        window.dispatch_event(WindowEvent::PointerReleased {
            position: pos,
            button: PointerEventButton::Left,
        });
        assert!(
            ui.get_paste_confirm_open(),
            "protected backdrop must not cancel paste"
        );
        key(&window, Key::Return);
        settle(&window);
        assert!(!ui.get_paste_confirm_open());
        assert_eq!(confirms.get(), 1);
    })
    .join()
    .unwrap();
}

#[test]
fn modal_keyboard_state_is_local_to_each_window() {
    std::thread::spawn(|| {
        let (first, first_window, windows) = host();
        let second = AppWindow::new().unwrap();
        second.show().unwrap();
        let second_window = windows.borrow()[1].clone();
        second_window.set_size(slint::PhysicalSize::new(1000, 850));
        first.set_shortcuts_open(true);
        second.set_about_open(true);
        settle(&first_window);
        settle(&second_window);
        key(&second_window, Key::Escape);
        settle(&second_window);
        assert!(!second.get_about_open());
        assert!(first.get_shortcuts_open());
        key(&first_window, Key::Escape);
        settle(&first_window);
        assert!(!first.get_shortcuts_open());
    })
    .join()
    .unwrap();
}

#[test]
fn detached_windows_share_cancel_keys_and_editor_guards_unsaved_changes() {
    std::thread::spawn(|| {
        let (_main, _, windows) = host();
        let process_ui = ProcWindow::new().unwrap();
        process_ui.show().unwrap();
        let process_window = windows.borrow()[1].clone();
        process_window.set_size(slint::PhysicalSize::new(1000, 850));
        let closes = Rc::new(Cell::new(0));
        let count = closes.clone();
        process_ui.on_close(move || count.set(count.get() + 1));
        settle(&process_window);
        key(&process_window, Key::Escape);
        assert_eq!(closes.get(), 1);

        let system_ui = SystemInfoWindow::new().unwrap();
        system_ui.show().unwrap();
        let system_window = windows.borrow()[2].clone();
        system_window.set_size(slint::PhysicalSize::new(1000, 850));
        let count = closes.clone();
        system_ui.on_close(move || count.set(count.get() + 1));
        settle(&system_window);
        key(&system_window, Key::Escape);
        assert_eq!(closes.get(), 2);

        let editor = EditorWindow::new().unwrap();
        editor.set_editor_open(true);
        editor.show().unwrap();
        let editor_window = windows.borrow()[3].clone();
        editor_window.set_size(slint::PhysicalSize::new(1000, 850));
        let count = closes.clone();
        editor.on_close_editor(move || count.set(count.get() + 1));
        settle(&editor_window);
        key(&editor_window, "x");
        assert_eq!(
            editor.get_editor_content(),
            "x",
            "editor must restore input focus"
        );
        assert!(editor.get_editor_dirty());
        key(&editor_window, Key::Escape);
        settle(&editor_window);
        assert!(editor.get_discard_confirm_open());
        assert_eq!(closes.get(), 2);
        key(&editor_window, Key::Return);
        assert!(
            editor.get_discard_confirm_open(),
            "Enter must not discard changes"
        );
        key(&editor_window, Key::Escape);
        settle(&editor_window);
        assert!(!editor.get_discard_confirm_open());
        assert_eq!(editor.get_editor_content(), "x");
        editor.set_editor_dirty(false);
        key(&editor_window, Key::Escape);
        assert_eq!(closes.get(), 3);
    })
    .join()
    .unwrap();
}

// Coordinates below are measured with the Windows UI font in a fixed-size fixture.
#[cfg(windows)]
#[test]
fn saved_secret_reveal_flows_through_the_real_app_window() {
    std::thread::spawn(|| {
        let (ui, window, _) = host();
        window.set_size(slint::PhysicalSize::new(1000, 1100));
        ui.set_ui_font_family("Microsoft YaHei".into());
        let mut saved = Session::new_empty();
        saved.id = "reveal-fixture".into();
        saved.host = "example.invalid".into();
        saved.password = Secret::new("fixture-only-password");
        let dir = std::env::temp_dir().join(format!("ms-reveal-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Rc::new(RefCell::new(crate::config::fixture_store(
            dir.join("sessions.json"),
            vec![saved.clone()],
        )));
        let mut outer = Session::new_empty();
        outer.id = "a".into();
        outer.host = "a.invalid".into();
        let mut inner = Session::new_empty();
        inner.id = "b".into();
        inner.host = "b.invalid".into();
        inner.jump_session_id = "a".into();
        store.borrow_mut().cache.sessions.extend([outer, inner]);
        store.borrow_mut().cache.sessions[0].jump_session_id = "b".into();
        wire_session_callbacks(
            &ui,
            99,
            store.clone(),
            Rc::new(WindowRegistry::default()),
            Default::default(),
            Default::default(),
            Default::default(),
            Rc::new(RefCell::new(crate::layout::Layout::new(vec![], "".into()))),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Arc::new(Runtime::new().unwrap()),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Rc::new(EditorWindow::new().unwrap()),
            Rc::new(RefCell::new(PassphraseState::new())),
        );
        ui.invoke_edit_session(saved.id.clone().into());
        let render = || {
            slint::platform::update_timers_and_animations();
            let mut pixels = vec![slint::Rgb8Pixel::default(); 1000 * 1100];
            window.request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, 1000);
            });
            slint::platform::update_timers_and_animations();
            pixels
        };
        let pixels = render();
        let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
        image::save_buffer(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/reveal-before.png"),
            &bytes,
            1000,
            1100,
            image::ColorType::Rgb8,
        )
        .unwrap();
        let click = |x, y| {
            let position = slint::LogicalPosition::new(x, y);
            window.dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            window.dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
            render();
        };
        let password_pixels = |pixels: &[slint::Rgb8Pixel]| {
            (575..635)
                .flat_map(|y| {
                    (312..650).map(move |x| {
                        let p = pixels[y * 1000 + x];
                        (p.r, p.g, p.b)
                    })
                })
                .collect::<Vec<_>>()
        };
        // The MCP-access row (#432) grows the centered dialog by 30px:
        // content above it sits 15px higher and the footer 15px lower.
        click(308., 653.);
        assert!(ui.get_dialog_allow_secret_reveal());
        assert_eq!(
            ui.get_dialog_password(),
            "fixture-only-password",
            "opt-in must load the saved value masked"
        );
        // Save without opening the eye: the permission must persist independently.
        click(660., 1015.);
        assert!(
            !ui.get_dialog_open(),
            "save failed: {}",
            ui.get_dialog_test_status()
        );
        assert!(ui.get_dialog_password().is_empty());
        let raw = std::fs::read_to_string(dir.join("sessions.json")).unwrap();
        let disk: crate::config::ConfigFile = serde_json::from_str(&raw).unwrap();
        assert!(
            disk.sessions
                .iter()
                .find(|s| s.id == saved.id)
                .unwrap()
                .allow_secret_reveal
        );
        assert!(!raw.contains("fixture-only-password"));
        ui.invoke_edit_session(saved.id.clone().into());
        let masked = password_pixels(&render());
        assert_eq!(ui.get_dialog_password(), "fixture-only-password");
        click(680., 605.);
        assert!(
            masked != password_pixels(&render()),
            "eye must visibly switch from dots to the saved text"
        );
        click(680., 605.);
        assert!(
            masked == password_pixels(&render()),
            "second click must hide the value again"
        );
        // Type a replacement through the real TextInput, then save and reopen.
        click(400., 605.);
        let control = if cfg!(target_os = "macos") {
            Key::Meta
        } else {
            Key::Control
        };
        window.dispatch_event(WindowEvent::KeyPressed {
            text: control.into(),
        });
        key(&window, "a");
        window.dispatch_event(WindowEvent::KeyReleased {
            text: control.into(),
        });
        for ch in "fixture-new-password".chars() {
            key(&window, ch.to_string());
        }
        assert_eq!(ui.get_dialog_password(), "fixture-new-password");
        ui.set_dialog_allow_secret_reveal(false);
        render();
        ui.set_dialog_allow_secret_reveal(true);
        render();
        assert_eq!(
            ui.get_dialog_password(),
            "fixture-new-password",
            "toggling opt-in must not overwrite a typed replacement"
        );
        click(660., 1015.);
        assert!(!ui.get_dialog_open(), "replacement save failed");
        ui.invoke_edit_session(saved.id.clone().into());
        let masked_new = password_pixels(&render());
        assert!(ui.get_dialog_allow_secret_reveal());
        assert_eq!(ui.get_dialog_password(), "fixture-new-password");
        click(680., 605.);
        assert!(masked_new != password_pixels(&render()));
        ui.set_dialog_allow_secret_reveal(false);
        render();
        click(660., 997.);
        assert!(!ui.get_dialog_open(), "opt-out save failed");
        ui.invoke_edit_session(saved.id.clone().into());
        render();
        assert!(!ui.get_dialog_allow_secret_reveal());
        assert!(ui.get_dialog_password().is_empty());
        assert!(ui.invoke_reveal_session_secret(false).is_empty());
        ui.invoke_session_dialog_cancel();
        render();
        assert!(ui.get_dialog_password().is_empty());
        let mut key_session = Session::new_empty();
        key_session.id = "key-fixture".into();
        key_session.host = "key.invalid".into();
        key_session.auth = AuthMethod::Key;
        key_session.password = Secret::new("fixture-passphrase");
        key_session.private_key_inline = Secret::new("fixture-inline-key");
        key_session.allow_secret_reveal = true;
        store.borrow_mut().upsert(key_session);
        store.borrow().save().unwrap();
        ui.invoke_edit_session("key-fixture".into());
        render();
        assert_eq!(ui.get_dialog_password(), "fixture-passphrase");
        assert_eq!(ui.get_dialog_key_inline(), "fixture-inline-key");
        ui.invoke_session_dialog_cancel();
        render();
        assert!(ui.get_dialog_key_inline().is_empty());
        let raw = std::fs::read_to_string(dir.join("sessions.json")).unwrap();
        assert!(!raw.contains("fixture-new-password") && !raw.contains("fixture-inline-key"));
        // Verify the newly typed value actually reached disk, not only the UI cache.
        use base64::Engine as _;
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            ChaCha20Poly1305, Nonce,
        };
        let disk: crate::config::ConfigFile = serde_json::from_str(&raw).unwrap();
        let password = &disk
            .sessions
            .iter()
            .find(|s| s.id == saved.id)
            .unwrap()
            .password;
        let encrypted = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(password.as_str().strip_prefix("enc:v1:").unwrap())
            .unwrap();
        let cipher = ChaCha20Poly1305::new_from_slice(&store.borrow().key).unwrap();
        let decrypted = cipher
            .decrypt(Nonce::from_slice(&encrypted[..12]), &encrypted[12..])
            .unwrap();
        assert_eq!(decrypted, b"fixture-new-password");
        let _ = std::fs::remove_dir_all(dir);
    })
    .join()
    .unwrap();
}

fn render_frame(window: &MinimalSoftwareWindow) -> Vec<slint::Rgb8Pixel> {
    let mut pixels = vec![slint::Rgb8Pixel::default(); 1000 * 850];
    window.request_redraw();
    assert!(window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, 1000);
    }));
    pixels
}

fn save_frame(pixels: &[slint::Rgb8Pixel], name: &str) {
    let bytes: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    std::fs::create_dir_all(&dir).unwrap();
    image::save_buffer(dir.join(name), &bytes, 1000, 850, image::ColorType::Rgb8).unwrap();
}

fn px(pixels: &[slint::Rgb8Pixel], x: i32, y: i32) -> (u8, u8, u8) {
    let p = pixels[(y * 1000 + x) as usize];
    (p.r, p.g, p.b)
}

/// Linux software-renderer check for the settings page and the list confirm.
/// It is not a macOS click test. It checks that a light panel stays near-white,
/// that About stays inside settings, and that the confirm card is the window-
/// centered light surface rather than a clipped sidebar fragment.
#[test]
fn light_settings_and_list_confirm_stay_readable() {
    std::thread::spawn(|| {
        let (ui, window, _) = host();
        crate::i18n::set_language("zh");
        crate::i18n::apply_to_slint();
        ui.set_lang_en(false);
        ui.set_dark_mode(false);
        ui.set_color_reference("sunrise".into());
        ui.set_interface_open(true);
        ui.set_settings_page("about".into());
        ui.set_app_version("0.8.1-test".into());
        settle(&window);
        assert!(!ui.get_about_open());
        assert!(ui.get_interface_open());
        let about = render_frame(&window);
        save_frame(&about, "light-about.png");
        let (r, g, b) = px(&about, 500, 400);
        assert!(
            r > 230 && g > 230 && b > 230,
            "light settings content should sit on a near-white panel, got {r},{g},{b}"
        );
        let dark_text = about
            .iter()
            .filter(|p| p.r < 50 && p.g < 50 && p.b < 50)
            .count();
        assert!(
            dark_text > 30,
            "light mode needs dark text, got {dark_text}"
        );
        let gold = about
            .iter()
            .filter(|p| p.r > 150 && p.g > 90 && p.g < 190 && p.b < 80)
            .count();
        assert!(gold > 5, "sunrise accent should be visible, got {gold}");

        ui.set_dark_mode(true);
        ui.set_color_reference("robbyrussell".into());
        settle(&window);
        let dark = render_frame(&window);
        save_frame(&dark, "dark-about.png");
        let (r, g, b) = px(&dark, 500, 400);
        assert!(
            r < 80 && g < 80 && b < 90,
            "dark settings content should sit on a dark panel, got {r},{g},{b}"
        );
        let green = dark
            .iter()
            .filter(|p| {
                let (r, g, b) = (p.r as u16, p.g as u16, p.b as u16);
                g > r + 40 && g > b + 40 && g > 80
            })
            .count();
        assert!(
            green > 5,
            "robbyrussell accent should be visible, got {green}"
        );

        ui.set_dark_mode(false);
        ui.set_color_reference("sunrise".into());
        ui.set_list_confirm_kind("group".into());
        ui.set_list_confirm_text("删除这个空分组？不会删除其中的会话。".into());
        ui.set_list_confirm_open(true);
        settle(&window);
        assert!(!ui.get_about_open());
        let kind = format!("{:?}", ui.get_active_modal());
        assert!(
            kind.to_ascii_lowercase().contains("list"),
            "list confirm should cover settings, got {kind}"
        );
        let confirm = render_frame(&window);
        save_frame(&confirm, "light-confirm.png");
        let (r, g, b) = px(&confirm, 500, 425);
        assert!(
            r > 230 && g > 230 && b > 230,
            "confirm card should be the light window surface, got {r},{g},{b}"
        );
    })
    .join()
    .unwrap();
}
