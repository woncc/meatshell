use super::{
    event_requires_immediate_ui, record_ingested_chunk, take_closed_event, INGEST_FRAME_BUDGET,
};
use crate::ssh::SessionEvent;

fn count_requests(chunk_lengths: &[usize]) -> (usize, usize) {
    let mut since_checkpoint = 0usize;
    let mut requests = 0usize;
    let mut dirty_since_request = false;
    for &chunk_len in chunk_lengths {
        dirty_since_request = true;
        if record_ingested_chunk(chunk_len, &mut since_checkpoint) {
            requests += 1;
            dirty_since_request = false;
        }
    }
    if dirty_since_request {
        requests += 1;
    }
    (requests, since_checkpoint)
}

#[test]
fn exact_frame_budget_chunks_do_not_add_an_empty_tail_request() {
    let (requests, remainder) = count_requests(&[INGEST_FRAME_BUDGET, INGEST_FRAME_BUDGET]);
    assert_eq!(requests, 2);
    assert_eq!(remainder, 0);
}

#[test]
fn a_partial_tail_gets_one_final_request() {
    let (requests, remainder) = count_requests(&[INGEST_FRAME_BUDGET, INGEST_FRAME_BUDGET, 1]);
    assert_eq!(requests, 3);
    assert_eq!(remainder, 1);
}

#[test]
fn checkpoint_budget_carries_across_input_events() {
    let mut since_checkpoint = 0usize;
    assert!(!record_ingested_chunk(
        INGEST_FRAME_BUDGET - 1,
        &mut since_checkpoint
    ));
    assert!(record_ingested_chunk(1, &mut since_checkpoint));
    assert_eq!(since_checkpoint, 0);
}

#[test]
fn an_oversized_output_event_stays_one_atomic_checkpoint() {
    let (requests, remainder) = count_requests(&[INGEST_FRAME_BUDGET * 2 + 1]);
    assert_eq!(requests, 1);
    assert_eq!(remainder, 1);
}

#[test]
fn routine_shell_metadata_does_not_disable_tail_pacing() {
    assert!(!event_requires_immediate_ui(&SessionEvent::CommandRan(
        "tail -n 1000000 app.log".into()
    )));
    assert!(!event_requires_immediate_ui(&SessionEvent::CwdChanged(
        "/var/log".into()
    )));
    assert!(event_requires_immediate_ui(&SessionEvent::Connected));
    assert!(event_requires_immediate_ui(&SessionEvent::Closed(
        "connection lost".into()
    )));
}

#[test]
fn rz_prompt_is_immediate_and_a_closed_window_cancels_it() {
    let (tx, mut rx) = tokio::sync::oneshot::channel::<Vec<std::path::PathBuf>>();
    let prompt = SessionEvent::RzUploadPrompt {
        responder: crate::ssh::RzFileResponder::new(tx),
    };
    assert!(
        event_requires_immediate_ui(&prompt),
        "an rz prompt must not wait behind terminal pacing"
    );

    let mut batch = vec![
        prompt,
        SessionEvent::Output("still buffered".into()),
        SessionEvent::Closed("window closed".into()),
    ];
    let closed = take_closed_event(&mut batch);
    assert!(matches!(closed, Some(SessionEvent::Closed(reason)) if reason == "window closed"));
    assert!(
        batch.is_empty(),
        "the prompt must not survive the closed window"
    );
    match rx.try_recv() {
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {}
        other => panic!("dropping the prompt should cancel the ssh wait, got {other:?}"),
    }
}

#[test]
fn rz_prompt_captures_the_delivery_window_before_the_worker() {
    let src = include_str!("../../../src/app/session_event.rs");
    let marker = "SessionEvent::RzUploadPrompt { responder } =>";
    let start = src.find(marker).expect("rz prompt arm");
    let arm = &src[start..];
    let arm = &arm[..arm.find("SessionEvent::HostKeyPrompt").expect("next arm")];
    let capture = arm
        .find("DialogOwner::of(")
        .expect("capture current window");
    let worker = arm.find("std::thread::spawn").expect("dialog worker");
    assert!(
        capture < worker,
        "the window handle has to be read on the UI thread before the worker runs"
    );
    assert!(arm.contains("pick_rz_upload_files"));
    assert!(!arm.contains("FileDialog::new"));
}

#[test]
fn closed_event_discards_stale_output_backlog() {
    let mut events = vec![
        SessionEvent::Output("old-1".into()),
        SessionEvent::Output("old-2".into()),
        SessionEvent::Closed("connection closed".into()),
        SessionEvent::Output("stale-after-close".into()),
    ];

    let closed = take_closed_event(&mut events);

    assert!(matches!(closed, Some(SessionEvent::Closed(reason)) if reason == "connection closed"));
    assert!(events.is_empty());
}
