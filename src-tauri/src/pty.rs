use std::{
    env,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::{MultiplexerMode, StartupConfig};

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub pid: Option<u32>,
    pub command: String,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtyDataPayload {
    pub data: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionExitPayload {
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WarningPayload {
    pub message: String,
}

pub fn emit_warning(app: &AppHandle, message: impl Into<String>) {
    let payload = WarningPayload {
        message: message.into(),
    };
    let _ = app.emit("warning", payload);
}

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

pub struct SessionManager {
    inner: Mutex<Option<PtySession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn start(
        &self,
        app: &AppHandle,
        startup: &StartupConfig,
        cols: u16,
        rows: u16,
    ) -> Result<SessionInfo, String> {
        self.stop()?;

        let resolved = resolve_startup_command(startup)?;
        if let Some(message) = resolved.warning.as_deref() {
            emit_warning(app, message);
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                cols: cols.max(2),
                rows: rows.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("failed to open PTY: {err}"))?;

        let mut command = CommandBuilder::new(&resolved.program);
        for arg in &resolved.args {
            command.arg(arg);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| format!("failed to spawn child process: {err}"))?;
        let pid = child.process_id();
        let killer = child.clone_killer();

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| format!("failed to clone PTY reader: {err}"))?;
        let app_for_reader = app.clone();
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        let data = String::from_utf8_lossy(&buffer[..size]).to_string();
                        let _ = app_for_reader.emit("pty:data", PtyDataPayload { data });
                    }
                    Err(_) => break,
                }
            }
        });

        let app_for_exit = app.clone();
        std::thread::spawn(move || {
            let code = child.wait().ok().map(|status| status.exit_code() as i32);
            let _ = app_for_exit.emit("session:exit", SessionExitPayload { code });
        });

        let writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("failed to take PTY writer: {err}"))?;

        let session = PtySession {
            master: pair.master,
            writer,
            killer,
        };

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "failed to lock session state".to_string())?;
        *guard = Some(session);

        Ok(SessionInfo {
            pid,
            command: resolved.display,
            fallback_used: resolved.fallback_used,
        })
    }

    pub fn write_stdin(&self, data: String) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "failed to lock session state".to_string())?;
        let session = guard
            .as_mut()
            .ok_or_else(|| "session is not running".to_string())?;

        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|err| format!("failed to write to PTY: {err}"))?;
        session
            .writer
            .flush()
            .map_err(|err| format!("failed to flush PTY writer: {err}"))?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "failed to lock session state".to_string())?;
        let session = guard
            .as_mut()
            .ok_or_else(|| "session is not running".to_string())?;
        session
            .master
            .resize(PtySize {
                cols: cols.max(2),
                rows: rows.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("failed to resize PTY: {err}"))
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "failed to lock session state".to_string())?;
        if let Some(mut session) = guard.take() {
            let _ = session.killer.kill();
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ResolvedCommand {
    program: String,
    args: Vec<String>,
    display: String,
    fallback_used: bool,
    warning: Option<String>,
}

fn resolve_startup_command(startup: &StartupConfig) -> Result<ResolvedCommand, String> {
    resolve_startup_command_with_checker(startup, command_exists)
}

fn resolve_startup_command_with_checker<F>(
    startup: &StartupConfig,
    command_exists_fn: F,
) -> Result<ResolvedCommand, String>
where
    F: Fn(&str) -> bool,
{
    match startup.multiplexer {
        MultiplexerMode::None => Ok(resolve_shell_command(startup, false, None)),
        MultiplexerMode::Tmux => {
            let (program, args) = parse_command_line(&startup.tmux_command)?;
            if command_exists_fn(&program) {
                Ok(ResolvedCommand {
                    display: join_command(&program, &args),
                    program,
                    args,
                    fallback_used: false,
                    warning: None,
                })
            } else {
                Ok(resolve_shell_command(
                    startup,
                    true,
                    Some(format!(
                        "tmux が見つからないため通常シェルで起動します: {}",
                        startup.tmux_command
                    )),
                ))
            }
        }
        MultiplexerMode::Zellij => {
            let (program, mut args) = parse_command_line(&startup.zellij_command)?;
            normalize_zellij_attach_args(&program, &mut args);
            if command_exists_fn(&program) {
                Ok(ResolvedCommand {
                    display: join_command(&program, &args),
                    program,
                    args,
                    fallback_used: false,
                    warning: None,
                })
            } else {
                Ok(resolve_shell_command(
                    startup,
                    true,
                    Some(format!(
                        "zellij が見つからないため通常シェルで起動します: {}",
                        startup.zellij_command
                    )),
                ))
            }
        }
    }
}

fn normalize_zellij_attach_args(program: &str, args: &mut Vec<String>) {
    if program != "zellij" {
        return;
    }

    if args.len() == 2 && args[0] == "attach" && args[1] == "-c" {
        args.push("d3term".to_string());
    }
}

fn resolve_shell_command(
    startup: &StartupConfig,
    fallback_used: bool,
    warning: Option<String>,
) -> ResolvedCommand {
    let shell = startup
        .shell
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(default_shell);

    let mut args = startup.shell_args.clone();
    if args.is_empty() {
        args.push("-l".to_string());
    }

    ResolvedCommand {
        display: join_command(&shell, &args),
        program: shell,
        args,
        fallback_used,
        warning,
    }
}

fn default_shell() -> String {
    env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "/bin/zsh".to_string())
}

fn parse_command_line(input: &str) -> Result<(String, Vec<String>), String> {
    let parts = shell_words::split(input).map_err(|err| format!("invalid command: {err}"))?;
    if parts.is_empty() {
        return Err("command is empty".to_string());
    }

    let program = parts[0].clone();
    let args = parts.into_iter().skip(1).collect::<Vec<_>>();
    Ok((program, args))
}

fn join_command(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        return program.to_string();
    }

    format!("{program} {}", args.join(" "))
}

fn command_exists(program: &str) -> bool {
    if program.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(program).exists();
    }

    match env::var_os("PATH") {
        Some(path) => env::split_paths(&path).any(|entry| entry.join(program).is_file()),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_line_splits_program_and_args() {
        let parsed =
            parse_command_line("zellij attach -c d3term").expect("parse should succeed");
        assert_eq!(parsed.0, "zellij");
        assert_eq!(parsed.1, vec!["attach", "-c", "d3term"]);
    }

    #[test]
    fn missing_multiplexer_falls_back_to_shell() {
        let startup = StartupConfig {
            multiplexer: MultiplexerMode::Zellij,
            shell: Some("/bin/zsh".to_string()),
            shell_args: vec!["-l".to_string()],
            zellij_command: "zellij attach -c d3term".to_string(),
            tmux_command: "tmux new-session -A -s main".to_string(),
        };
        let resolved = resolve_startup_command_with_checker(&startup, |_program| false)
            .expect("fallback should resolve");
        assert_eq!(resolved.program, "/bin/zsh");
        assert!(resolved.fallback_used);
    }

    #[test]
    fn none_mode_uses_shell() {
        let startup = StartupConfig {
            multiplexer: MultiplexerMode::None,
            shell: Some("/bin/bash".to_string()),
            shell_args: vec![],
            zellij_command: "zellij attach -c d3term".to_string(),
            tmux_command: "tmux new-session -A -s main".to_string(),
        };
        let resolved = resolve_startup_command_with_checker(&startup, |_program| true)
            .expect("shell mode should resolve");
        assert_eq!(resolved.program, "/bin/bash");
        assert_eq!(resolved.args, vec!["-l"]);
    }

    #[test]
    fn legacy_zellij_default_is_upgraded_with_session_name() {
        let startup = StartupConfig {
            multiplexer: MultiplexerMode::Zellij,
            shell: None,
            shell_args: vec![],
            zellij_command: "zellij attach -c".to_string(),
            tmux_command: "tmux new-session -A -s main".to_string(),
        };
        let resolved = resolve_startup_command_with_checker(&startup, |_program| true)
            .expect("zellij command should resolve");
        assert_eq!(resolved.program, "zellij");
        assert_eq!(resolved.args, vec!["attach", "-c", "d3term"]);
    }
}
