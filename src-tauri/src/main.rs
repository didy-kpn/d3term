#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod pty;
mod state;

use commands::{resize, start_session, stop_session, write_stdin};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(state::AppState::new())
        .setup(|app| {
            let state = app.state::<state::AppState>();

            if let Err(err) = state.config.start_watch(app.handle().clone()) {
                eprintln!("failed to start config watcher: {err}");
            }
            if let Err(err) = state.config.emit_current(app.handle()) {
                eprintln!("failed to emit initial config: {err}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            write_stdin,
            resize,
            stop_session
        ])
        .run(tauri::generate_context!())
        .expect("failed to run d3term");
}
