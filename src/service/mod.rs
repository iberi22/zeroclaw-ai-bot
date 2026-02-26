use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const SERVICE_LABEL: &str = "com.zeroclaw.daemon";
const WINDOWS_TASK_NAME: &str = "ZeroClaw Daemon";
const WINDOWS_RUN_REG_VALUE: &str = "ZeroClawDaemon";

fn windows_task_name() -> &'static str {
    WINDOWS_TASK_NAME
}

fn windows_run_reg_value() -> &'static str {
    WINDOWS_RUN_REG_VALUE
}

pub fn handle_command(command: &crate::ServiceCommands, config: &Config) -> Result<()> {
    match command {
        crate::ServiceCommands::Install => install(config),
        crate::ServiceCommands::Start => start(config),
        crate::ServiceCommands::Stop => stop(config),
        crate::ServiceCommands::Status => status(config),
        crate::ServiceCommands::Uninstall => uninstall(config),
    }
}

fn install(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        install_macos(config)
    } else if cfg!(target_os = "linux") {
        install_linux(config)
    } else if cfg!(target_os = "windows") {
        install_windows(config)
    } else {
        anyhow::bail!("Service management is supported on macOS and Linux only");
    }
}

fn start(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        let plist = macos_service_file()?;
        run_checked(Command::new("launchctl").arg("load").arg("-w").arg(&plist))?;
        run_checked(Command::new("launchctl").arg("start").arg(SERVICE_LABEL))?;
        println!("✅ Service started");
        Ok(())
    } else if cfg!(target_os = "linux") {
        run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
        run_checked(Command::new("systemctl").args(["--user", "start", "zeroclaw.service"]))?;
        println!("✅ Service started");
        Ok(())
    } else if cfg!(target_os = "windows") {
        let wrapper = windows_wrapper_path(config);
        match run_checked(Command::new("schtasks").args(["/Run", "/TN", windows_task_name()])) {
            Ok(()) => {
                println!("✅ Service started");
                Ok(())
            }
            Err(task_err) => {
                if windows_run_key_exists() {
                    start_windows_wrapper_detached(&wrapper)?;
                    println!("✅ Service started (startup-registry mode)");
                    Ok(())
                } else {
                    Err(task_err)
                }
            }
        }
    } else {
        let _ = config;
        anyhow::bail!("Service management is supported on macOS and Linux only")
    }
}

fn stop(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        let plist = macos_service_file()?;
        let _ = run_checked(Command::new("launchctl").arg("stop").arg(SERVICE_LABEL));
        let _ = run_checked(
            Command::new("launchctl")
                .arg("unload")
                .arg("-w")
                .arg(&plist),
        );
        println!("✅ Service stopped");
        Ok(())
    } else if cfg!(target_os = "linux") {
        let _ = run_checked(Command::new("systemctl").args(["--user", "stop", "zeroclaw.service"]));
        println!("✅ Service stopped");
        Ok(())
    } else if cfg!(target_os = "windows") {
        let _ = config;
        let task_name = windows_task_name();
        let _ = run_checked(Command::new("schtasks").args(["/End", "/TN", task_name]));
        let _ = stop_windows_daemon_processes();
        println!("✅ Service stopped");
        Ok(())
    } else {
        let _ = config;
        anyhow::bail!("Service management is supported on macOS and Linux only")
    }
}

fn status(config: &Config) -> Result<()> {
    if cfg!(target_os = "macos") {
        let out = run_capture(Command::new("launchctl").arg("list"))?;
        let running = out.lines().any(|line| line.contains(SERVICE_LABEL));
        println!(
            "Service: {}",
            if running {
                "✅ running/loaded"
            } else {
                "❌ not loaded"
            }
        );
        println!("Unit: {}", macos_service_file()?.display());
        return Ok(());
    }

    if cfg!(target_os = "linux") {
        let out = run_capture(Command::new("systemctl").args([
            "--user",
            "is-active",
            "zeroclaw.service",
        ]))
        .unwrap_or_else(|_| "unknown".into());
        println!("Service state: {}", out.trim());
        println!("Unit: {}", linux_service_file(config)?.display());
        return Ok(());
    }

    if cfg!(target_os = "windows") {
        let wrapper = windows_wrapper_path(config);
        let task_name = windows_task_name();
        let task_query = Command::new("schtasks")
            .args(["/Query", "/TN", task_name, "/FO", "LIST"])
            .output();
        match task_query {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                let task_running = text.contains("Running");
                let fallback_running = windows_daemon_running().unwrap_or(false);
                let effective_running = task_running || fallback_running;
                let mode = if task_running {
                    "scheduled-task mode"
                } else if fallback_running {
                    "startup-registry mode"
                } else {
                    "stopped"
                };
                println!(
                    "Service: {}",
                    if effective_running {
                        "✅ running"
                    } else {
                        "❌ not running"
                    }
                );
                println!("Mode: {}", mode);
                println!("Task: {}", task_name);
                if windows_run_key_exists() {
                    println!("Startup entry: HKCU\\...\\Run\\{}", windows_run_reg_value());
                    println!("Wrapper: {}", wrapper.display());
                }
            }
            _ => {
                if windows_run_key_exists() {
                    let running = windows_daemon_running().unwrap_or(false);
                    println!(
                        "Service: {}",
                        if running {
                            "✅ running (startup-registry mode)"
                        } else {
                            "❌ not running (startup-registry mode)"
                        }
                    );
                    println!("Startup entry: HKCU\\...\\Run\\{}", windows_run_reg_value());
                    println!("Wrapper: {}", wrapper.display());
                } else {
                    println!("Service: ❌ not installed");
                }
            }
        }
        return Ok(());
    }

    anyhow::bail!("Service management is supported on macOS and Linux only")
}

fn uninstall(config: &Config) -> Result<()> {
    stop(config)?;

    if cfg!(target_os = "macos") {
        let file = macos_service_file()?;
        if file.exists() {
            fs::remove_file(&file)
                .with_context(|| format!("Failed to remove {}", file.display()))?;
        }
        println!("✅ Service uninstalled ({})", file.display());
        return Ok(());
    }

    if cfg!(target_os = "linux") {
        let file = linux_service_file(config)?;
        if file.exists() {
            fs::remove_file(&file)
                .with_context(|| format!("Failed to remove {}", file.display()))?;
        }
        let _ = run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]));
        println!("✅ Service uninstalled ({})", file.display());
        return Ok(());
    }

    if cfg!(target_os = "windows") {
        let task_name = windows_task_name();
        let _ = run_checked(Command::new("schtasks").args(["/Delete", "/TN", task_name, "/F"]));
        let _ = uninstall_windows_run_key();
        // Remove the wrapper script
        let wrapper = windows_wrapper_path(config);
        if wrapper.exists() {
            fs::remove_file(&wrapper).ok();
        }
        println!("✅ Service uninstalled");
        return Ok(());
    }

    anyhow::bail!("Service management is supported on macOS and Linux only")
}

fn install_macos(config: &Config) -> Result<()> {
    let file = macos_service_file()?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let logs_dir = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs");
    fs::create_dir_all(&logs_dir)?;

    let stdout = logs_dir.join("daemon.stdout.log");
    let stderr = logs_dir.join("daemon.stderr.log");

    let plist = format!(
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>daemon</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{stdout}</string>
  <key>StandardErrorPath</key>
  <string>{stderr}</string>
</dict>
</plist>
"#,
        label = SERVICE_LABEL,
        exe = xml_escape(&exe.display().to_string()),
        stdout = xml_escape(&stdout.display().to_string()),
        stderr = xml_escape(&stderr.display().to_string())
    );

    fs::write(&file, plist)?;
    println!("✅ Installed launchd service: {}", file.display());
    println!("   Start with: zeroclaw service start");
    Ok(())
}

fn install_linux(config: &Config) -> Result<()> {
    let file = linux_service_file(config)?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let unit = format!(
        "[Unit]\nDescription=ZeroClaw daemon\nAfter=network.target\n\n[Service]\nType=simple\nExecStart={} daemon\nRestart=always\nRestartSec=3\n\n[Install]\nWantedBy=default.target\n",
        exe.display()
    );

    fs::write(&file, unit)?;
    let _ = run_checked(Command::new("systemctl").args(["--user", "daemon-reload"]));
    let _ = run_checked(Command::new("systemctl").args(["--user", "enable", "zeroclaw.service"]));
    println!("✅ Installed systemd user service: {}", file.display());
    println!("   Start with: zeroclaw service start");
    Ok(())
}

fn install_windows(config: &Config) -> Result<()> {
    let exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let logs_dir = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs");
    fs::create_dir_all(&logs_dir)?;

    // Create a wrapper script that redirects output to log files
    let wrapper = windows_wrapper_path(config);
    let stdout_log = logs_dir.join("daemon.stdout.log");
    let stderr_log = logs_dir.join("daemon.stderr.log");
    let supervisor_log = logs_dir.join("daemon.supervisor.log");
    let state_file = config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("daemon_state.json");

    let wrapper_content = build_windows_supervisor_wrapper(
        &exe.display().to_string(),
        &state_file.display().to_string(),
        &stdout_log.display().to_string(),
        &stderr_log.display().to_string(),
        &supervisor_log.display().to_string(),
    );
    fs::write(&wrapper, &wrapper_content)?;

    let task_name = windows_task_name();

    // Remove any existing task first (ignore errors if it doesn't exist)
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", task_name, "/F"])
        .output();

    let task_command = format!("\"{}\"", wrapper.display());
    let create_with_level = |run_level: &str| -> Result<()> {
        run_checked(Command::new("schtasks").args([
            "/Create",
            "/TN",
            task_name,
            "/SC",
            "ONLOGON",
            "/TR",
            &task_command,
            "/RL",
            run_level,
            "/F",
        ]))
    };

    let install_result = create_with_level("HIGHEST")
        .or_else(|e| {
            let lower = e.to_string().to_ascii_lowercase();
            let access_denied = lower.contains("access is denied")
                || lower.contains("acceso denegado")
                || lower.contains("permiso denegado");
            if access_denied {
                create_with_level("LIMITED")
            } else {
                Err(e)
            }
        })
        .map(|_| "task");

    match install_result {
        Ok(_) => {
            println!("✅ Installed Windows scheduled task: {}", task_name);
            println!("   Wrapper: {}", wrapper.display());
            println!("   Logs: {}", logs_dir.display());
            println!("   Restart policy: always (supervisor loop, 5s delay)");
            println!("   Start with: zeroclaw service start");
            Ok(())
        }
        Err(task_err) => {
            install_windows_run_key(&wrapper)?;
            println!(
                "✅ Installed Windows startup registry entry: HKCU\\...\\Run\\{}",
                windows_run_reg_value()
            );
            println!("   Wrapper: {}", wrapper.display());
            println!("   Logs: {}", logs_dir.display());
            println!("   Restart policy: always (supervisor loop, 5s delay)");
            println!("   Note: scheduled task unavailable ({task_err})");
            println!("   Start with: zeroclaw service start");
            Ok(())
        }
    }
}

fn windows_wrapper_path(config: &Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("logs")
        .join("zeroclaw-daemon.cmd")
}

fn install_windows_run_key(wrapper: &std::path::Path) -> Result<()> {
    run_checked(Command::new("reg").args([
        "ADD",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "/V",
        windows_run_reg_value(),
        "/T",
        "REG_SZ",
        "/D",
        &format!("\"{}\"", wrapper.display()),
        "/F",
    ]))
}

fn uninstall_windows_run_key() -> Result<()> {
    run_checked(Command::new("reg").args([
        "DELETE",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "/V",
        windows_run_reg_value(),
        "/F",
    ]))
}

fn windows_run_key_exists() -> bool {
    Command::new("reg")
        .args([
            "QUERY",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/V",
            windows_run_reg_value(),
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn start_windows_wrapper_detached(wrapper: &std::path::Path) -> Result<()> {
    run_checked(Command::new("cmd").args([
        "/C",
        "start",
        "\"ZeroClaw Daemon\"",
        "/MIN",
        &wrapper.display().to_string(),
    ]))
}

fn stop_windows_daemon_processes() -> Result<()> {
    run_checked(Command::new("powershell").args([
        "-NoProfile",
        "-Command",
        "Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'zeroclaw.exe' -and $_.CommandLine -match ' daemon(\\s|$)' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }",
    ]))
}

fn windows_daemon_running() -> Result<bool> {
    let out = run_capture(Command::new("powershell").args([
        "-NoProfile",
        "-Command",
        "(@(Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'zeroclaw.exe' -and $_.CommandLine -match ' daemon(\\s|$)' })).Count",
    ]))?;
    let count = out.trim().parse::<u32>().unwrap_or(0);
    Ok(count > 0)
}

fn build_windows_supervisor_wrapper(
    exe_path: &str,
    state_file: &str,
    stdout_log: &str,
    stderr_log: &str,
    supervisor_log: &str,
) -> String {
    format!(
        "@echo off\r\n\
setlocal EnableExtensions EnableDelayedExpansion\r\n\
set \"ZEROCLAW_EXE={exe_path}\"\r\n\
set \"ZEROCLAW_STATE={state_file}\"\r\n\
set \"ZEROCLAW_STDOUT={stdout_log}\"\r\n\
set \"ZEROCLAW_STDERR={stderr_log}\"\r\n\
set \"ZEROCLAW_SUPERVISOR={supervisor_log}\"\r\n\
set /a RESTART_COUNT=0\r\n\
:run_loop\r\n\
set /a RESTART_COUNT+=1\r\n\
echo [%date% %time%] starting daemon attempt !RESTART_COUNT!>>\"!ZEROCLAW_SUPERVISOR!\"\r\n\
if exist \"!ZEROCLAW_STATE!\" del /f /q \"!ZEROCLAW_STATE!\" >nul 2>&1\r\n\
\"!ZEROCLAW_EXE!\" daemon >>\"!ZEROCLAW_STDOUT!\" 2>>\"!ZEROCLAW_STDERR!\"\r\n\
set \"EXIT_CODE=!errorlevel!\"\r\n\
echo [%date% %time%] daemon exited with code !EXIT_CODE!; restarting in 5s>>\"!ZEROCLAW_SUPERVISOR!\"\r\n\
timeout /t 5 /nobreak >nul\r\n\
goto run_loop\r\n"
    )
}

fn macos_service_file() -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{SERVICE_LABEL}.plist")))
}

fn linux_service_file(config: &Config) -> Result<PathBuf> {
    let home = directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .context("Could not find home directory")?;
    let _ = config;
    Ok(home
        .join(".config")
        .join("systemd")
        .join("user")
        .join("zeroclaw.service"))
}

fn run_checked(command: &mut Command) -> Result<()> {
    let output = command.output().context("Failed to spawn command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr.trim());
    }
    Ok(())
}

fn run_capture(command: &mut Command) -> Result<String> {
    let output = command.output().context("Failed to spawn command")?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    Ok(text)
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_escapes_reserved_chars() {
        let escaped = xml_escape("<&>\"' and text");
        assert_eq!(escaped, "&lt;&amp;&gt;&quot;&apos; and text");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_capture_reads_stdout() {
        let out = run_capture(Command::new("sh").args(["-lc", "echo hello"]))
            .expect("stdout capture should succeed");
        assert_eq!(out.trim(), "hello");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_capture_falls_back_to_stderr() {
        let out = run_capture(Command::new("sh").args(["-lc", "echo warn 1>&2"]))
            .expect("stderr capture should succeed");
        assert_eq!(out.trim(), "warn");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_checked_errors_on_non_zero_status() {
        let err = run_checked(Command::new("sh").args(["-lc", "exit 17"]))
            .expect_err("non-zero exit should error");
        assert!(err.to_string().contains("Command failed"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn linux_service_file_has_expected_suffix() {
        let file = linux_service_file(&Config::default()).unwrap();
        let path = file.to_string_lossy();
        assert!(path.ends_with(".config/systemd/user/zeroclaw.service"));
    }

    #[test]
    fn windows_task_name_is_constant() {
        assert_eq!(windows_task_name(), "ZeroClaw Daemon");
    }

    #[test]
    fn windows_supervisor_wrapper_has_restart_loop_and_state_reset() {
        let wrapper = build_windows_supervisor_wrapper(
            "C:\\bin\\zeroclaw.exe",
            "C:\\data\\daemon_state.json",
            "C:\\logs\\daemon.stdout.log",
            "C:\\logs\\daemon.stderr.log",
            "C:\\logs\\daemon.supervisor.log",
        );
        assert!(wrapper.contains(":run_loop"));
        assert!(wrapper.contains("goto run_loop"));
        assert!(wrapper.contains("daemon exited with code"));
        assert!(wrapper.contains("del /f /q"));
        assert!(wrapper.contains("timeout /t 5 /nobreak"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_capture_reads_stdout_windows() {
        let out = run_capture(Command::new("cmd").args(["/C", "echo hello"]))
            .expect("stdout capture should succeed");
        assert_eq!(out.trim(), "hello");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn run_checked_errors_on_non_zero_status_windows() {
        let err = run_checked(Command::new("cmd").args(["/C", "exit /b 17"]))
            .expect_err("non-zero exit should error");
        assert!(err.to_string().contains("Command failed"));
    }
}
