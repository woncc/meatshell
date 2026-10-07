use super::*;

#[test]
fn marks_owner_and_preserves_source_tab() {
    let input = vec![
        ProcInfo {
            pid: 10,
            user: "alice".into(),
            cpu: 1.0,
            mem: 2.0,
            rss_kib: 512,
            command: "own".into(),
        },
        ProcInfo {
            pid: 11,
            user: "root".into(),
            cpu: 3.0,
            mem: 4.0,
            rss_kib: 2048,
            command: "other".into(),
        },
    ];
    let rows = proc_rows(&input, "alice", "term-a", 0, false);
    assert!(rows[0].own_process);
    assert!(!rows[1].own_process);
    assert!(rows.iter().all(|row| row.tab_id.as_str() == "term-a"));
}

#[test]
fn privilege_rules_match_effective_login_user() {
    assert!(!process_needs_root("alice", "alice"));
    assert!(process_needs_root("alice", "root"));
    assert!(process_needs_root("alice", "bob"));
    assert!(!process_needs_root("root", "root"));
    assert!(!process_needs_root("root", "alice"));
}

#[test]
fn sorts_each_column_and_formats_resident_size() {
    let input = vec![
        ProcInfo {
            pid: 2,
            user: "bob".into(),
            cpu: 1.0,
            mem: 9.0,
            rss_kib: 1024,
            command: "zeta".into(),
        },
        ProcInfo {
            pid: 1,
            user: "Alice".into(),
            cpu: 5.0,
            mem: 1.0,
            rss_kib: 1536,
            command: "alpha".into(),
        },
        ProcInfo {
            pid: 3,
            user: "alice".into(),
            cpu: 5.0,
            mem: 1.0,
            rss_kib: 1,
            command: "Alpha".into(),
        },
    ];

    let cpu_desc = proc_rows(&input, "alice", "t", 2, true);
    // Same CPU: lower pid stays first because the tie-break is not reversed.
    assert_eq!(cpu_desc[0].pid.as_str(), "1");
    assert_eq!(cpu_desc[1].pid.as_str(), "3");
    assert_eq!(cpu_desc[0].mem_size.as_str(), "1.5 MB");
    assert_eq!(cpu_desc[2].mem_size.as_str(), "1.0 MB");
    assert_eq!(
        proc_rows(&input, "alice", "t", 4, false)[0]
            .mem_size
            .as_str(),
        "1.0 KB"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 4, true)[0].pid.as_str(),
        "1"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 3, false)[0].pid.as_str(),
        "1"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 3, true)[0].pid.as_str(),
        "2"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 0, true)[0].pid.as_str(),
        "3"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 1, false)[0].user.as_str(),
        "Alice"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 1, false)[1].user.as_str(),
        "alice"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 5, true)[0].command.as_str(),
        "zeta"
    );
    assert_eq!(
        proc_rows(&input, "alice", "t", 5, false)[0]
            .command
            .as_str(),
        "alpha"
    );
    assert_eq!(
        proc_rows(
            &[ProcInfo {
                pid: 9,
                user: "root".into(),
                cpu: 0.0,
                mem: 0.0,
                rss_kib: 0,
                command: "idle".into(),
            }],
            "root",
            "t",
            4,
            false,
        )[0]
        .mem_size
        .as_str(),
        "0 B"
    );
    // Unknown column falls back to CPU, still descending when asked.
    assert_eq!(
        proc_rows(&input, "alice", "t", 99, true)[0].pid.as_str(),
        "1"
    );
}
