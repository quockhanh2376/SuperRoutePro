#[cfg(target_os = "windows")]
use crate::win32_consts::CREATE_NO_WINDOW;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_CMD_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_POWERSHELL_TIMEOUT_SECS: u64 = 45;
pub const NETWORK_COMMAND_TIMEOUT_SECS: u64 = 90;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn run_process_blocking(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;

    let started = Instant::now();
    loop {
        match child
            .try_wait()
            .map_err(|e| format!("Failed waiting for {}: {}", program, e))?
        {
            Some(_) => {
                return child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to collect output for {}: {}", program, e));
            }
            None => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Command timed out after {}s: {} {}",
                        timeout.as_secs(),
                        program,
                        args.join(" ")
                    ));
                }
                std::thread::sleep(Duration::from_millis(80));
            }
        }
    }
}

pub fn run_powershell_blocking(script: &str, timeout: Duration) -> Result<String, String> {
    let output = run_process_blocking(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
        timeout,
    )?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if !stdout.is_empty() {
            Ok(stdout)
        } else {
            Err(stderr)
        }
    }
}

pub fn run_cmd_blocking(program: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
    let output = run_process_blocking(program, args, timeout)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else if !stdout.is_empty() {
        Ok(format!("{}\n{}", stdout, stderr))
    } else {
        Err(stderr)
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
