#[cfg(target_os = "windows")]
use crate::win32_consts::CREATE_NO_WINDOW;
use std::io::Read;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_CMD_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_POWERSHELL_TIMEOUT_SECS: u64 = 45;
pub const NETWORK_COMMAND_TIMEOUT_SECS: u64 = 90;
const POST_EXIT_PIPE_DRAIN_GRACE: Duration = Duration::from_millis(250);

struct PipeDrain {
    state: Arc<(Mutex<(Vec<u8>, bool)>, Condvar)>,
}

impl PipeDrain {
    fn spawn<R: Read + Send + 'static>(pipe: Option<R>) -> Self {
        let state = Arc::new((Mutex::new((Vec::new(), false)), Condvar::new()));
        let worker_state = Arc::clone(&state);

        std::thread::spawn(move || {
            if let Some(mut reader) = pipe {
                let mut chunk = [0u8; 4096];
                loop {
                    match reader.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(bytes_read) => {
                            let mut guard = worker_state
                                .0
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner());
                            guard.0.extend_from_slice(&chunk[..bytes_read]);
                        }
                        Err(_) => break,
                    }
                }
            }

            let mut guard = worker_state
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.1 = true;
            worker_state.1.notify_all();
        });

        Self { state }
    }

    fn collect_until(&self, deadline: Instant) -> Vec<u8> {
        let mut guard = self
            .state
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        while !guard.1 {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let (next_guard, wait_result) = self
                .state
                .1
                .wait_timeout(guard, deadline.saturating_duration_since(now))
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard = next_guard;
            if wait_result.timed_out() {
                break;
            }
        }

        guard.0.clone()
    }
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
pub(crate) fn command_exists_on_path(name: &str) -> bool {
    command_exists_on_path_value(name, std::env::var("PATH").ok().as_deref())
}

#[cfg(target_os = "windows")]
pub(crate) fn command_exists_on_path_value(name: &str, path_var: Option<&str>) -> bool {
    let exe_name = if name.contains('.') {
        name.to_string()
    } else {
        format!("{name}.exe")
    };

    path_var
        .unwrap_or_default()
        .split(';')
        .filter(|entry| !entry.trim().is_empty())
        .any(|dir| Path::new(dir.trim()).join(&exe_name).is_file())
}

pub fn run_process_blocking(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;

    let stdout_drain = PipeDrain::spawn(child.stdout.take());
    let stderr_drain = PipeDrain::spawn(child.stderr.take());

    let started = Instant::now();
    let overall_deadline = started + timeout;
    loop {
        match child
            .try_wait()
            .map_err(|e| format!("Failed waiting for {}: {}", program, e))?
        {
            Some(status) => {
                let drain_deadline = (Instant::now() + POST_EXIT_PIPE_DRAIN_GRACE).min(overall_deadline);
                let stdout = stdout_drain.collect_until(drain_deadline);
                let stderr = stderr_drain.collect_until(drain_deadline);
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            None => {
                if Instant::now() >= overall_deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Command timed out after {}s: {} {}",
                        timeout.as_secs(),
                        program,
                        args.join(" ")
                    ));
                }
                std::thread::sleep(Duration::from_millis(15));
            }
        }
    }
}

pub fn run_hidden_output_blocking(program: &str, args: &[&str]) -> Result<Output, String> {
    run_process_blocking(program, args, Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS))
}

pub fn run_hidden_stdout_blocking(program: &str, args: &[&str]) -> Result<String, String> {
    let output = run_hidden_output_blocking(program, args)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn ps_escape_single_quoted(input: &str) -> String {
    input.replace('\'', "''")
}

fn format_command_failure(stdout: &str, stderr: &str, status: ExitStatus) -> String {
    let stdout_trimmed = stdout.trim();
    let stderr_trimmed = stderr.trim();
    match (stdout_trimmed.is_empty(), stderr_trimmed.is_empty()) {
        (false, false) => format!("{stdout_trimmed}\n{stderr_trimmed}"),
        (false, true) => stdout_trimmed.to_string(),
        (true, false) => stderr_trimmed.to_string(),
        (true, true) => format!("Command exited with status {status}"),
    }
}

pub fn run_powershell_blocking(script: &str, timeout: Duration) -> Result<String, String> {
    let output = run_process_blocking(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
        timeout,
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format_command_failure(&stdout, &stderr, output.status))
    }
}

pub fn run_cmd_blocking(program: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
    let output = run_process_blocking(program, args, timeout)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format_command_failure(&stdout, &stderr, output.status))
    }
}

pub async fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let program_owned = program.to_string();
    let args_owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    tauri::async_runtime::spawn_blocking(move || {
        let args_refs: Vec<&str> = args_owned.iter().map(String::as_str).collect();
        run_cmd_blocking(
            &program_owned,
            &args_refs,
            Duration::from_secs(DEFAULT_CMD_TIMEOUT_SECS),
        )
    })
    .await
    .map_err(|err| format!("Command task join error: {}", err))?
}

pub async fn run_powershell(script: &str) -> Result<String, String> {
    let script_owned = script.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        run_powershell_blocking(
            &script_owned,
            Duration::from_secs(DEFAULT_POWERSHELL_TIMEOUT_SECS),
        )
    })
    .await
    .map_err(|err| format!("PowerShell task join error: {}", err))?
}

pub async fn run_process(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let program_owned = program.to_string();
    let args_owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    tauri::async_runtime::spawn_blocking(move || {
        let args_refs: Vec<&str> = args_owned.iter().map(String::as_str).collect();
        run_process_blocking(&program_owned, &args_refs, timeout)
    })
    .await
    .map_err(|err| format!("Process task join error: {}", err))?
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::{
        command_exists_on_path_value, run_cmd_blocking, run_powershell_blocking,
        run_process_blocking,
    };
    use super::ps_escape_single_quoted;
    #[cfg(target_os = "windows")]
    use std::fs;
    #[cfg(target_os = "windows")]
    use std::time::{Duration, Instant};

    #[test]
    fn single_quote_escaping_doubles_embedded_quotes() {
        assert_eq!(ps_escape_single_quoted("Contoso's App"), "Contoso''s App");
        assert_eq!(ps_escape_single_quoted("plain"), "plain");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn command_exists_on_path_value_honors_semicolon_entries_and_appends_exe() {
        let temp_root = std::env::temp_dir().join("srp-process-exec-path-test");
        let tool_dir = temp_root.join("bin");
        fs::create_dir_all(&tool_dir).expect("temp tool dir should be creatable");
        fs::write(tool_dir.join("demo-tool.exe"), b"stub")
            .expect("temp exe stub should be creatable");

        let path_value = tool_dir.to_string_lossy().to_string();
        assert!(command_exists_on_path_value("demo-tool", Some(&path_value)));
        assert!(command_exists_on_path_value(
            "demo-tool.exe",
            Some(&path_value)
        ));
        assert!(!command_exists_on_path_value(
            "demo-tool",
            Some(r"D:\missing-dir")
        ));

        fs::remove_dir_all(&temp_root).expect("temp tool dir should be removable");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_process_blocking_drains_large_stdout_without_deadlock() {
        let started = Instant::now();
        let output = run_process_blocking(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Out.Write('A' * 163840)",
            ],
            Duration::from_secs(10),
        )
        .expect("large stdout process should complete without timing out");

        assert!(output.status.success());
        assert!(
            output.stdout.len() >= 131_072,
            "expected >128 KB stdout, got {} bytes",
            output.stdout.len()
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "large stdout process took too long: {:?}",
            started.elapsed()
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_cmd_blocking_returns_err_with_stdout_and_stderr_on_nonzero_exit() {
        let err = run_cmd_blocking(
            "cmd",
            &["/C", "echo stdout-failure-detail & echo stderr-failure-detail 1>&2 & exit /b 1"],
            Duration::from_secs(5),
        )
        .expect_err("non-zero exit code must return Err even when stdout is non-empty");

        assert!(
            err.contains("stdout-failure-detail"),
            "error should include stdout, got: {err}"
        );
        assert!(
            err.contains("stderr-failure-detail"),
            "error should include stderr, got: {err}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_powershell_blocking_returns_err_with_stdout_and_stderr_on_nonzero_exit() {
        let err = run_powershell_blocking(
            "Write-Output 'ps-stdout-detail'; [Console]::Error.WriteLine('ps-stderr-detail'); exit 1",
            Duration::from_secs(10),
        )
        .expect_err("non-zero powershell exit code must return Err even when stdout is non-empty");

        assert!(
            err.contains("ps-stdout-detail"),
            "error should include stdout, got: {err}"
        );
        assert!(
            err.contains("ps-stderr-detail"),
            "error should include stderr, got: {err}"
        );
    }
}
