use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, ModelRc, Rgb8Pixel, VecModel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

slint::slint! {
    import { Welcome, SessionInfo } from "../ui/welcome.slint";
    import "../ui/fonts/MaterialIcons-Regular.ttf";
    export component GroupFixture inherits Window {
        width: 320px;
        height: 440px;
        in property <[SessionInfo]> sessions;
        callback moved(string, string);
        callback began();
        Welcome {
            width: 100%; height: 100%;
            compact: true;
            sessions: root.sessions;
            move-session(id, group) => { root.moved(id, group); }
            reorder-begin => { root.began(); }
        }
    }
}

struct Backend(Rc<MinimalSoftwareWindow>);
impl slint::platform::Platform for Backend {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn render(window: &MinimalSoftwareWindow) -> Vec<Rgb8Pixel> {
    let mut pixels = vec![Rgb8Pixel::default(); 320 * 440];
    for _ in 0..2 {
        slint::platform::update_timers_and_animations();
        window.request_redraw();
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, 320);
        });
    }
    pixels
}

fn moved(window: &MinimalSoftwareWindow, y: f32) {
    window.dispatch_event(WindowEvent::PointerMoved {
        position: slint::LogicalPosition::new(100.0, y),
    });
    render(window);
}

#[test]
fn group_drag_targets_survive_startup_fix() {
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Backend(window.clone()))).unwrap();
        let ui = GroupFixture::new().unwrap();
        let rows: ModelRc<SessionInfo> = Rc::new(VecModel::from(vec![
            SessionInfo {
                id: "local".into(),
                name: "PowerShell".into(),
                group: "system".into(),
                group_header: "system".into(),
                builtin: true,
                ..Default::default()
            },
            SessionInfo {
                id: "source".into(),
                name: "Example".into(),
                group: "source-group".into(),
                group_header: "source-group".into(),
                ..Default::default()
            },
            SessionInfo {
                group: "target-group".into(),
                group_header: "target-group".into(),
                ..Default::default()
            },
        ]))
        .into();
        ui.set_sessions(rows);
        let moves = Rc::new(RefCell::new(Vec::new()));
        let calls = moves.clone();
        ui.on_moved(move |id, group| calls.borrow_mut().push((id.to_string(), group.to_string())));
        let began = Rc::new(Cell::new(false));
        let flag = began.clone();
        ui.on_began(move || flag.set(true));
        ui.show().unwrap();
        window.set_size(slint::PhysicalSize::new(320, 440));
        render(&window);
        // These coordinates come from the rendered compact fixture: the saved
        // host is at y=313, target-group at y=354 and system at y=205.
        let drag = |destination: f32, leave_target: bool| {
            moves.borrow_mut().clear();
            began.set(false);
            moved(&window, 313.0);
            window.dispatch_event(WindowEvent::PointerPressed {
                position: slint::LogicalPosition::new(100.0, 313.0),
                button: PointerEventButton::Left,
            });
            moved(&window, 330.0);
            assert!(began.get(), "the pointer must actually start a host drag");
            moved(&window, destination);
            let release_y = if leave_target { 414.0 } else { destination };
            if leave_target {
                moved(&window, release_y);
            }
            window.dispatch_event(WindowEvent::PointerReleased {
                position: slint::LogicalPosition::new(100.0, release_y),
                button: PointerEventButton::Left,
            });
            render(&window);
            let result = moves.borrow().clone();
            result
        };
        assert_eq!(
            drag(354.0, false),
            vec![("source".to_owned(), "target-group".to_owned())]
        );
        assert!(
            drag(354.0, true).is_empty(),
            "leaving the header must clear the target"
        );
        assert!(
            drag(205.0, false).is_empty(),
            "system must never accept a host drop"
        );
    })
    .join()
    .unwrap();
}
