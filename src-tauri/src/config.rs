use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex, RwLock},
    thread,
    time::{Duration, Instant},
};

use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::pty::emit_warning;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MultiplexerMode {
    None,
    Tmux,
    Zellij,
}

impl Default for MultiplexerMode {
    fn default() -> Self {
        Self::Zellij
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    System,
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct StartupConfig {
    pub multiplexer: MultiplexerMode,
    pub shell: Option<String>,
    pub shell_args: Vec<String>,
    pub zellij_command: String,
    pub tmux_command: String,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            multiplexer: MultiplexerMode::Zellij,
            shell: None,
            shell_args: Vec::new(),
            zellij_command: "zellij attach -c d3term".to_string(),
            tmux_command: "tmux new-session -A -s main".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TerminalConfig {
    pub theme: ThemeMode,
    pub font_family: String,
    pub font_size: f64,
    pub letter_spacing: f64,
    pub line_height: f64,
    pub scrollback: u32,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            font_family: "'JetBrains Mono', Menlo, monospace".to_string(),
            font_size: 13.0,
            letter_spacing: 0.0,
            line_height: 1.2,
            scrollback: 10_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub startup: StartupConfig,
    pub terminal: TerminalConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            startup: StartupConfig::default(),
            terminal: TerminalConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigUpdatedPayload {
    pub config: AppConfig,
    pub path: String,
}

pub struct ConfigManager {
    path: PathBuf,
    config: Arc<RwLock<AppConfig>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        let path = resolve_config_path();
        let config = match load_config_from_path(&path) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("failed to load config at {}: {err}", path.display());
                AppConfig::default()
            }
        };

        Self {
            path,
            config: Arc::new(RwLock::new(config)),
            watcher: Mutex::new(None),
        }
    }

    pub fn current(&self) -> AppConfig {
        self.config
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| AppConfig::default())
    }

    pub fn emit_current(&self, app: &AppHandle) -> Result<(), String> {
        let payload = ConfigUpdatedPayload {
            config: self.current(),
            path: self.path.display().to_string(),
        };
        app.emit("config:updated", payload)
            .map_err(|err| err.to_string())
    }

    pub fn start_watch(&self, app: AppHandle) -> Result<(), String> {
        let mut watcher_guard = self
            .watcher
            .lock()
            .map_err(|_| "failed to lock config watcher".to_string())?;
        if watcher_guard.is_some() {
            return Ok(());
        }

        let watch_root = resolve_watch_root(&self.path);
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            move |event_result| {
                let _ = tx.send(event_result);
            },
            NotifyConfig::default(),
        )
        .map_err(|err| format!("failed to create config watcher: {err}"))?;

        watcher
            .watch(&watch_root, RecursiveMode::Recursive)
            .map_err(|err| format!("failed to watch {}: {err}", watch_root.display()))?;

        let config_path = self.path.clone();
        let config_cell = Arc::clone(&self.config);
        thread::spawn(move || watch_loop(app, rx, config_path, config_cell));

        *watcher_guard = Some(watcher);
        Ok(())
    }
}

fn watch_loop(
    app: AppHandle,
    rx: mpsc::Receiver<notify::Result<notify::Event>>,
    config_path: PathBuf,
    config_cell: Arc<RwLock<AppConfig>>,
) {
    let mut last_reload_at = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .unwrap_or_else(Instant::now);

    while let Ok(event_result) = rx.recv() {
        if last_reload_at.elapsed() < Duration::from_millis(200) {
            continue;
        }
        last_reload_at = Instant::now();

        match event_result {
            Ok(_event) => match load_config_from_path(&config_path) {
                Ok(next) => {
                    let mut changed = false;
                    if let Ok(mut current) = config_cell.write() {
                        if *current != next {
                            *current = next.clone();
                            changed = true;
                        }
                    }
                    if changed {
                        let payload = ConfigUpdatedPayload {
                            config: next,
                            path: config_path.display().to_string(),
                        };
                        let _ = app.emit("config:updated", payload);
                    }
                }
                Err(err) => emit_warning(&app, format!("設定の再読込に失敗しました: {err}")),
            },
            Err(err) => emit_warning(&app, format!("設定ファイル監視エラー: {err}")),
        }
    }
}

pub fn resolve_config_path() -> PathBuf {
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();
    resolve_config_path_with_env(xdg.as_deref(), home.as_deref())
}

pub fn resolve_config_path_with_env(xdg: Option<&str>, home: Option<&str>) -> PathBuf {
    if let Some(xdg_home) = xdg {
        let xdg_home = xdg_home.trim();
        if !xdg_home.is_empty() {
            return PathBuf::from(xdg_home).join("d3term").join("config.toml");
        }
    }

    if let Some(home_dir) = home {
        let home_dir = home_dir.trim();
        if !home_dir.is_empty() {
            return PathBuf::from(home_dir)
                .join(".config")
                .join("d3term")
                .join("config.toml");
        }
    }

    PathBuf::from(".config").join("d3term").join("config.toml")
}

fn resolve_watch_root(config_path: &Path) -> PathBuf {
    if let Some(parent) = config_path.parent() {
        if parent.exists() {
            return parent.to_path_buf();
        }

        if let Some(grand_parent) = parent.parent() {
            if grand_parent.exists() {
                return grand_parent.to_path_buf();
            }
        }
    }

    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn load_config_from_path(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let raw = fs::read_to_string(path).map_err(|err| format!("read error: {err}"))?;
    toml::from_str::<AppConfig>(&raw).map_err(|err| format!("parse error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_uses_xdg_if_available() {
        let path = resolve_config_path_with_env(Some("/tmp/xdg"), Some("/tmp/home"));
        assert_eq!(path, PathBuf::from("/tmp/xdg/d3term/config.toml"));
    }

    #[test]
    fn resolve_path_falls_back_to_home() {
        let path = resolve_config_path_with_env(None, Some("/tmp/home"));
        assert_eq!(path, PathBuf::from("/tmp/home/.config/d3term/config.toml"));
    }

    #[test]
    fn parse_with_partial_fields_uses_defaults() {
        let raw = r#"
            [startup]
            multiplexer = "tmux"

            [terminal]
            font_size = 15
            letter_spacing = -1
        "#;

        let parsed = toml::from_str::<AppConfig>(raw).expect("should parse");
        assert_eq!(parsed.startup.multiplexer, MultiplexerMode::Tmux);
        assert_eq!(parsed.terminal.font_size, 15.0);
        assert_eq!(parsed.terminal.letter_spacing, -1.0);
        assert_eq!(parsed.terminal.line_height, 1.2);
    }

    #[test]
    fn invalid_toml_is_error() {
        let raw = "startup = [";
        let parsed = toml::from_str::<AppConfig>(raw);
        assert!(parsed.is_err());
    }
}
