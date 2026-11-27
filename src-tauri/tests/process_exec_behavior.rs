#[cfg(target_os = "windows")]
#[test]
fn run_process_blocking_does_not_hang_past_timeout_when_grandchild_holds_pipe() {
    use std::time::{Duration, Instant};
    use super_route_pro_lib::process_exec::run_process_blocking;

    let started = Instant::now();
    let result = run_process_blocking(
        "cmd",
        &["/C", "start /B ping -n 5 127.0.0.1"],
        Duration::from_secs(1),
    );
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(3),
        "run_process_blocking hung for {elapsed:?} despite 1s timeout (result={result:?})"
    );
}
