//! Live COM smoke test. Runs against the local WMI service, so it only makes
//! sense on Windows and needs the local WMI service (winmgmt) up — which is
//! the default. We query for PID 4 (System / System Idle Process) because
//! it exists on every Windows install we care about.
//!
//! The test is opt-in via `--ignored` because a) CI Linux boxes can't run it,
//! b) locked-down WMI can return E_ACCESSDENIED, and we don't want the
//! default `cargo test` to fail for reasons unrelated to this crate.

#[cfg(windows)]
#[test]
#[ignore = "live COM: run with `cargo test -- --ignored` on Windows"]
fn win32_process_pid4_smoke() {
    use windows_wmi_com::{Wmi, WmiValue};

    let wmi = Wmi::connect_local("ROOT\\CIMV2").expect("connect");
    let rows: Vec<_> = wmi
        .query_raw("SELECT Name, ProcessId FROM Win32_Process WHERE ProcessId = 4")
        .expect("exec")
        .collect();

    assert!(!rows.is_empty(), "expected at least one row for PID 4");
    let row = rows.into_iter().next().unwrap().expect("row");

    match row.get("Name") {
        Some(WmiValue::String(s)) => {
            assert!(!s.is_empty(), "Name should not be empty");
        }
        other => panic!("Name should be a non-empty string, got {:?}", other),
    }

    match row.get("ProcessId") {
        Some(WmiValue::U32(4)) => {}
        other => panic!("ProcessId should be U32(4), got {:?}", other),
    }
}

#[cfg(not(windows))]
#[test]
fn non_windows_placeholder() {
    // Deliberately does nothing — the crate is Windows-only.
}
