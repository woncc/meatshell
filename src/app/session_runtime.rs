use super::*;

/// When a firehose queue contains stale output ahead of its terminal `Closed`
/// event, keep the close notification and discard the obsolete output. The
/// disconnected tab is going to be reset anyway; replaying those bytes only
/// delays cleanup and can temporarily retain hundreds of megabytes.
pub(super) fn take_closed_event(events: &mut Vec<SessionEvent>) -> Option<SessionEvent> {
    let index = events
        .iter()
        .position(|event| matches!(event, SessionEvent::Closed(_)))?;
    let closed = events.swap_remove(index);
    events.clear();
    Some(closed)
}

pub(super) fn resolve_jump(
    store: &Rc<RefCell<ConfigStore>>,
    session: &Session,
) -> Result<Vec<Session>> {
    store.borrow().resolve_jump_chain(session)
}

pub(super) fn should_start_sftp(session: &Session, sftp_enabled: bool) -> bool {
    // Shell-integration compatibility must not hide SFTP. Auto-login scripts
    // can require the shell hooks to be disabled while still using SFTP.
    // The settings master switch is the only global off switch.
    sftp_enabled && session.kind == SessionKind::Ssh
}

/// Spawn the shell (+ SFTP) workers and their event-pump threads for an
/// already-registered tab. Used by the initial connect and by in-place
/// reconnect (#79); the tab/terminal/parser must already exist.
pub(super) fn start_session_in_tab(tab_id: &str, session: Session, ctx: &ConnectCtx) {
    let sftp_enabled = ctx.store.borrow().sftp_enabled();
    let has_sftp = should_start_sftp(&session, sftp_enabled);
    sync_terminal_sftp_availability(ctx, tab_id, has_sftp);
    let (initial_cols, initial_rows) = *ctx.last_term_size.lock().unwrap();
    // Resolve every ancestor on the UI thread before starting any worker.
    // Invalid chains must never silently fall back to a direct connection.
    let jump = match resolve_jump(&ctx.store, &session) {
        Ok(jump) => jump,
        Err(error) => {
            if let (Some(win), Some(editor)) = (ctx.weak.upgrade(), ctx.editor.upgrade()) {
                apply_session_event_to_window(
                    &win,
                    &editor,
                    ctx.window_id,
                    tab_id,
                    SessionEvent::Closed(error.to_string()),
                    &ctx.bufs,
                    &ctx.render_gates,
                    &ctx.tab_statuses,
                    &ctx.local_snap,
                    &ctx.local_net_hist,
                );
            }
            return;
        }
    };
    let (handle, rx) = match session.kind {
        SessionKind::Ssh => spawn_session(
            ctx.runtime.handle(),
            tab_id.to_string(),
            session.clone(),
            jump.clone(),
            initial_cols,
            initial_rows,
        ),
        SessionKind::Serial => crate::terminal::serial::spawn_serial_session(
            ctx.runtime.handle(),
            tab_id.to_string(),
            session.clone(),
        ),
        SessionKind::Telnet => crate::terminal::telnet::spawn_telnet_session(
            ctx.runtime.handle(),
            tab_id.to_string(),
            session.clone(),
            initial_cols,
            initial_rows,
        ),
        SessionKind::Local => crate::terminal::local::spawn_local_session(
            ctx.runtime.handle(),
            tab_id.to_string(),
            session.clone(),
            initial_cols,
            initial_rows,
        ),
        SessionKind::Rdp => {
            // RDP sessions are handed to the system remote desktop client and
            // never get a tab (see `on_connect_session`), so this arm is only a
            // safety net for a saved session edited into another kind.
            tracing::warn!("RDP session cannot be hosted in a tab; not starting");
            return;
        }
    };
    let terminal_reply_tx = handle.commands.clone();
    let monitoring_enabled = ctx
        .weak
        .upgrade()
        .map(|window| !window.get_sidebar_collapsed() && !window.get_zen_mode())
        .unwrap_or(true);
    handle.set_resource_monitoring(monitoring_enabled);
    ctx.handles.borrow_mut().insert(tab_id.to_string(), handle);

    // Delivery route for this tab. Both pump threads hold the Arc and
    // re-read it on every batch, so a later detach/merge only rewrites the
    // route — the pumps keep running and target the new window (#tab-detach).
    let route = Arc::new(Mutex::new(TabRoute {
        window: ctx.weak.clone(),
        editor: ctx.editor.clone(),
        window_id: ctx.window_id,
        bufs: ctx.bufs.clone(),
        gates: ctx.render_gates.clone(),
        statuses: ctx.tab_statuses.clone(),
        local_snap: ctx.local_snap.clone(),
        net_hist: ctx.local_net_hist.clone(),
        sftp_handles: ctx.sftp_handles.clone(),
        sftp_last_cwd: ctx.sftp_last_cwd.clone(),
        follow_cd: ctx.sftp_follow_cd.clone(),
        sftp_events: None,
    }));
    if let Ok(mut routes) = ctx.tab_routes.lock() {
        routes.insert(tab_id.to_string(), route.clone());
    }

    // SSH tabs keep an SFTP event channel even when the master switch is off,
    // so turning SFTP back on can attach a worker without rebuilding the pump.
    // The worker itself is started only while the switch is on, and only after
    // the interactive PTY reports Connected.
    let (sftp_evt_tx, sftp_ready_tx) = if session.kind == SessionKind::Ssh {
        let (sftp_tx, sftp_rx) = tokio::sync::mpsc::unbounded_channel::<SessionEvent>();
        if let Ok(mut guard) = route.lock() {
            guard.sftp_events = Some(sftp_tx.clone());
        }
        let ready_tx = if has_sftp {
            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();
            let sftp_runtime = ctx.runtime.clone();
            let sftp_task_runtime = sftp_runtime.clone();
            let sftp_route = route.clone();
            let sftp_tab_id = tab_id.to_string();
            let sftp_enabled_flag = ctx.sftp_enabled.clone();
            sftp_runtime.spawn(async move {
                if !matches!(
                    tokio::time::timeout(std::time::Duration::from_secs(30), ready_rx).await,
                    Ok(Ok(()))
                ) {
                    return;
                }
                tokio::task::yield_now().await;
                // The master switch may have been turned off while this
                // handshake was in flight. Do not open the subsystem then.
                if !sftp_enabled_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                let sftp_handle = spawn_sftp(sftp_task_runtime.handle(), session, jump, sftp_tx);
                if !sftp_enabled_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    sftp_handle.close();
                    sftp_handle.join.abort();
                    return;
                }
                let handles = sftp_route.lock().ok().map(|r| r.sftp_handles.clone());
                if let Some(handles) = handles {
                    if let Ok(mut handles) = handles.lock() {
                        if handles.contains_key(&sftp_tab_id)
                            || !sftp_enabled_flag.load(std::sync::atomic::Ordering::Relaxed)
                        {
                            drop(handles);
                            sftp_handle.close();
                            sftp_handle.join.abort();
                            return;
                        }
                        handles.insert(sftp_tab_id, sftp_handle);
                    }
                }
            });
            Some(ready_tx)
        } else {
            None
        };
        (Some(sftp_rx), ready_tx)
    } else {
        (None, None)
    };

    // --- Shell event pump (dedicated thread) ---
    {
        let route_pump = route.clone();
        let rt_pump = ctx.runtime.clone();
        let tab_id_pump = tab_id.to_string();
        std::thread::spawn(move || {
            let mut shell_rx = rx;
            let mut sftp_ready_tx = sftp_ready_tx;
            let mut cwd_debounce: Option<tokio::task::JoinHandle<()>> = None;
            // Reusable scratch so a fast firehose doesn't reallocate every batch.
            let mut drained: Vec<SessionEvent> = Vec::new();
            // This survives drain batches, so a stream of small events cannot
            // evade the frame checkpoint merely because of thread timing.
            let mut ingested_since_checkpoint = 0usize;
            loop {
                // Block for the first event, then sweep up everything else that's
                // already queued. A burst — e.g. `tail -f` on a busy log (#171) —
                // then collapses into ONE invoke_from_event_loop and (after merging
                // adjacent Output below) ONE vt100 ingest + render, instead of one
                // UI task per chunk flooding the event loop and freezing the app.
                match shell_rx.blocking_recv() {
                    None => break,
                    Some(first) => drained.push(first),
                }
                // Cap the sweep so an unending stream still yields to the renderer
                // between batches (keeps the UI live rather than starved).
                const DRAIN_CAP: usize = 2048;
                while drained.len() < DRAIN_CAP {
                    match shell_rx.try_recv() {
                        Ok(evt) => drained.push(evt),
                        Err(_) => break,
                    }
                }

                // Resolve the current delivery route once per batch: a tab that
                // was detached/merged mid-stream simply delivers this batch
                // to its new window (#tab-detach).
                let Ok(rt) = route_pump.lock().map(|g| g.clone()) else {
                    continue;
                };

                // A close marker can sit behind a large burst of Output events
                // in the unbounded channel. Handle it before ingesting anything
                // from this batch so stale scrollback is released immediately.
                if let Some(closed) = take_closed_event(&mut drained) {
                    if let Some(h) = crate::app::term_buf(&rt.bufs, &tab_id_pump) {
                        h.lock().unwrap().release_history_keep_screen();
                    }
                    let rt_evt = rt.clone();
                    let tid = tab_id_pump.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let (Some(win), Some(editor)) =
                            (rt_evt.window.upgrade(), rt_evt.editor.upgrade())
                        {
                            apply_session_event_to_window(
                                &win,
                                &editor,
                                rt_evt.window_id,
                                &tid,
                                closed,
                                &rt_evt.bufs,
                                &rt_evt.gates,
                                &rt_evt.statuses,
                                &rt_evt.local_snap,
                                &rt_evt.net_hist,
                            );
                        }
                    });
                    break;
                }

                // Run CwdChanged side-effects here (off the UI thread), drop the
                // swallowed ones, and concatenate runs of Output into a single chunk
                // so the UI parses + renders the whole burst once.
                let mut ui_batch: Vec<SessionEvent> = Vec::with_capacity(drained.len());
                for evt in drained.drain(..) {
                    match evt {
                        SessionEvent::Connected => {
                            if let Some(ready) = sftp_ready_tx.take() {
                                let _ = ready.send(());
                            }
                            ui_batch.push(SessionEvent::Connected);
                        }
                        SessionEvent::CwdChanged(cwd) => {
                            // Shared map (not a thread-local) so manual SFTP
                            // navigation can clear the entry — then the very next
                            // OSC 7, same directory or not, snaps the panel back to
                            // the shell's cwd. Unchanged repeats (every prompt
                            // re-emits OSC 7) are ignored (#59).
                            let changed = match rt.sftp_last_cwd.lock() {
                                Ok(mut m) => {
                                    m.insert(tab_id_pump.clone(), cwd.clone()).as_deref()
                                        != Some(cwd.as_str())
                                }
                                Err(_) => false,
                            };
                            // Swallow when follow-cd is off: forwarding it would set
                            // sftp_loading without any ListDir to clear it (the #59
                            // stuck-"loading" trap).
                            if !changed || !rt.follow_cd.load(std::sync::atomic::Ordering::Relaxed)
                            {
                                continue;
                            }
                            if let Some(prev) = cwd_debounce.take() {
                                prev.abort();
                            }
                            let cwd_spawn = cwd.clone();
                            let sftp_h = rt.sftp_handles.clone();
                            let tid = tab_id_pump.clone();
                            cwd_debounce = Some(rt_pump.spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                if let Ok(handles) = sftp_h.lock() {
                                    if let Some(h) = handles.get(&tid) {
                                        h.list_dir(cwd_spawn);
                                    }
                                }
                            }));
                            ui_batch.push(SessionEvent::CwdChanged(cwd));
                        }
                        SessionEvent::Output(chunk) => {
                            // Merge with the immediately preceding Output so the
                            // whole run is one vt100 ingest + one render. Only
                            // *adjacent* chunks merge, so byte order (and any
                            // interleaved event) is preserved exactly. Cap the
                            // merged size so one batch can't monopolize the UI
                            // thread for hundreds of ms (#209).
                            if let Some(SessionEvent::Output(prev)) = ui_batch.last_mut() {
                                if prev.len() + chunk.len() <= OUTPUT_MERGE_BYTE_CAP {
                                    prev.push_str(&chunk);
                                } else {
                                    ui_batch.push(SessionEvent::Output(chunk));
                                }
                            } else {
                                ui_batch.push(SessionEvent::Output(chunk));
                            }
                        }
                        other => ui_batch.push(other),
                    }
                }
                if ui_batch.is_empty() {
                    continue;
                }

                // Ingest terminal output on this pump thread (not the UI thread).
                // Keep each Output event atomic: TermBuffer detects full-screen
                // redraw sequences within one ingest call, so artificial byte
                // splits could corrupt scrollback when they bisect such a refresh.
                let mut remaining_output_bytes: usize = ui_batch
                    .iter()
                    .map(|event| match event {
                        SessionEvent::Output(chunk) => chunk.len(),
                        _ => 0,
                    })
                    .sum();
                let has_immediate_ui_events = ui_batch.iter().any(event_requires_immediate_ui);
                let mut dirty_since_request = false;
                let mut ui_only: Vec<SessionEvent> = Vec::with_capacity(ui_batch.len());
                for evt in ui_batch {
                    match evt {
                        SessionEvent::Output(chunk) => {
                            let chunk_len = chunk.len();
                            let reply =
                                ingest_terminal_output(&rt.bufs, &tab_id_pump, chunk.as_bytes());
                            if !reply.is_empty() {
                                let _ = terminal_reply_tx.send(SessionCommand::RawInput(reply));
                            }
                            remaining_output_bytes =
                                remaining_output_bytes.saturating_sub(chunk_len);
                            dirty_since_request = true;

                            if record_ingested_chunk(chunk_len, &mut ingested_since_checkpoint) {
                                let ticket = request_tab_render(
                                    rt.window.clone(),
                                    &tab_id_pump,
                                    &rt.bufs,
                                    &rt.gates,
                                );
                                dirty_since_request = false;

                                // The event channel is intentionally unbounded
                                // today. Waiting while a large backlog exists would
                                // only move bytes from the terminal buffer into that
                                // channel and inflate memory, so catch up first and
                                // pace once the stream's tail is within reach.
                                if !has_immediate_ui_events
                                    && remaining_output_bytes <= PACED_LOCAL_BACKLOG_LIMIT
                                    && shell_rx.len() <= PACED_QUEUE_EVENT_LIMIT
                                {
                                    wait_for_ui_flush(ticket);
                                }
                            }
                        }
                        other => ui_only.push(other),
                    }
                }

                if dirty_since_request {
                    let _ =
                        request_tab_render(rt.window.clone(), &tab_id_pump, &rt.bufs, &rt.gates);
                }

                if ui_only.is_empty() {
                    continue;
                }

                let rt_evt = rt.clone();
                let tid = tab_id_pump.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let (Some(win), Some(editor)) =
                        (rt_evt.window.upgrade(), rt_evt.editor.upgrade())
                    {
                        for evt in ui_only {
                            apply_session_event_to_window(
                                &win,
                                &editor,
                                rt_evt.window_id,
                                &tid,
                                evt,
                                &rt_evt.bufs,
                                &rt_evt.gates,
                                &rt_evt.statuses,
                                &rt_evt.local_snap,
                                &rt_evt.net_hist,
                            );
                        }
                    }
                });
            }
        });
    }

    // --- SFTP event pump (separate thread, SSH only) ---
    if let Some(sftp_evt_tx) = sftp_evt_tx {
        let route_sftp = route.clone();
        let tab_id_sftp = tab_id.to_string();
        std::thread::spawn(move || {
            let mut sftp_rx = sftp_evt_tx;
            let mut drained: Vec<SessionEvent> = Vec::new();
            loop {
                match sftp_rx.blocking_recv() {
                    None => break,
                    Some(first) => drained.push(first),
                }
                const SFTP_DRAIN_CAP: usize = 256;
                while drained.len() < SFTP_DRAIN_CAP {
                    match sftp_rx.try_recv() {
                        Ok(evt) => drained.push(evt),
                        Err(_) => break,
                    }
                }
                let ui_batch: Vec<SessionEvent> = drained.drain(..).collect();
                if ui_batch.is_empty() {
                    continue;
                }
                let Ok(rt_s) = route_sftp.lock().map(|g| g.clone()) else {
                    continue;
                };
                let tid = tab_id_sftp.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let (Some(win), Some(editor)) =
                        (rt_s.window.upgrade(), rt_s.editor.upgrade())
                    {
                        for sftp_evt in ui_batch {
                            apply_session_event_to_window(
                                &win,
                                &editor,
                                rt_s.window_id,
                                &tid,
                                sftp_evt,
                                &rt_s.bufs,
                                &rt_s.gates,
                                &rt_s.statuses,
                                &rt_s.local_snap,
                                &rt_s.net_hist,
                            );
                        }
                    }
                });
            }
        });
    }
}

fn sync_terminal_sftp_availability(ctx: &ConnectCtx, tab_id: &str, has_sftp: bool) {
    let Some(win) = ctx.weak.upgrade() else {
        return;
    };
    let terminals = win.get_terminals();
    let Some(model) = terminals.as_any().downcast_ref::<VecModel<TerminalState>>() else {
        return;
    };
    let collapse_default = win.get_collapse_sftp_default();
    crate::app::panes::update_terminal_row(model, tab_id, |row| {
        let was_available = row.sftp_available;
        row.sftp_available = has_sftp;
        if has_sftp && !was_available {
            row.sftp_collapsed = collapse_default;
        } else if !has_sftp {
            row.sftp_collapsed = true;
        }
    });
}

/// Close every live SFTP worker, or attach one for each connected SSH tab
/// that does not have one yet. The cd-follow preference is not touched.
pub(super) fn apply_sftp_master_switch(core: &crate::app::core::AppCore, enabled: bool) {
    if !enabled {
        let states = core.window_states.borrow();
        for state in states.values() {
            if let Ok(mut handles) = state.sftp_handles.lock() {
                for handle in handles.values() {
                    handle.close();
                    handle.join.abort();
                }
                handles.clear();
            }
        }
        return;
    }

    struct PendingSftp {
        tab_id: String,
        events: tokio::sync::mpsc::UnboundedSender<SessionEvent>,
        handles: crate::sftp::SftpHandles,
        window: slint::Weak<AppWindow>,
        session: crate::config::Session,
        jump: Vec<crate::config::Session>,
    }
    let mut pending = Vec::new();
    {
        let Ok(routes) = core.tab_routes.lock() else {
            return;
        };
        let store = core.store.borrow();
        for (tab_id, route_lock) in routes.iter() {
            let Ok(route) = route_lock.lock() else {
                continue;
            };
            let Some(events) = route.sftp_events.clone() else {
                continue;
            };
            let already = route
                .sftp_handles
                .lock()
                .map(|handles| handles.contains_key(tab_id))
                .unwrap_or(true);
            if already {
                continue;
            }
            let Some(status) = route
                .statuses
                .lock()
                .ok()
                .and_then(|statuses| statuses.get(tab_id).cloned())
            else {
                continue;
            };
            if status.state != 1 || status.session_id.is_empty() {
                continue;
            }
            let Some(session) = store.get(&status.session_id).cloned() else {
                continue;
            };
            if !should_start_sftp(&session, true) {
                continue;
            }
            let Ok(jump) = store.resolve_jump_chain(&session) else {
                continue;
            };
            pending.push(PendingSftp {
                tab_id: tab_id.clone(),
                events,
                handles: route.sftp_handles.clone(),
                window: route.window.clone(),
                session,
                jump,
            });
        }
    }
    for job in pending {
        let handle = spawn_sftp(core.runtime.handle(), job.session, job.jump, job.events);
        if let Ok(mut handles) = job.handles.lock() {
            handles.insert(job.tab_id.clone(), handle);
        }
        let Some(win) = job.window.upgrade() else {
            continue;
        };
        let terminals = win.get_terminals();
        let Some(model) = terminals.as_any().downcast_ref::<VecModel<TerminalState>>() else {
            continue;
        };
        let collapse_default = win.get_collapse_sftp_default();
        crate::app::panes::update_terminal_row(model, &job.tab_id, |row| {
            let was_available = row.sftp_available;
            row.sftp_available = true;
            if !was_available {
                row.sftp_collapsed = collapse_default;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::should_start_sftp;
    use crate::config::{Session, SessionKind};

    #[test]
    fn shell_compatibility_keeps_sftp_available() {
        let mut session = Session::new_empty();
        session.kind = SessionKind::Ssh;
        assert!(should_start_sftp(&session, true));

        session.disable_shell_integration = true;
        assert!(should_start_sftp(&session, true));
    }

    #[test]
    fn non_ssh_sessions_never_start_sftp() {
        let mut session = Session::new_empty();
        session.kind = SessionKind::Telnet;
        assert!(!should_start_sftp(&session, true));
    }

    #[test]
    fn master_switch_blocks_sftp_even_for_ssh() {
        let mut session = Session::new_empty();
        session.kind = SessionKind::Ssh;
        assert!(!should_start_sftp(&session, false));
        session.disable_shell_integration = true;
        assert!(!should_start_sftp(&session, false));
    }
}
