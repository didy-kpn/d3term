use tauri::{AppHandle, State};

use crate::{pty::SessionInfo, state::AppState};

#[tauri::command]
pub fn start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    cols: u16,
    rows: u16,
) -> Result<SessionInfo, String> {
    let config = state.config.current();
    state.config.emit_current(&app)?;
    state.session.start(&app, &config.startup, cols, rows)
}

#[tauri::command]
pub fn write_stdin(state: State<'_, AppState>, data: String) -> Result<(), String> {
    state.session.write_stdin(data)
}

#[tauri::command]
pub fn resize(state: State<'_, AppState>, cols: u16, rows: u16) -> Result<(), String> {
    state.session.resize(cols, rows)
}

#[tauri::command]
pub fn stop_session(state: State<'_, AppState>) -> Result<(), String> {
    state.session.stop()
}
