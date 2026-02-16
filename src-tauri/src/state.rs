use crate::{config::ConfigManager, pty::SessionManager};

pub struct AppState {
    pub config: ConfigManager,
    pub session: SessionManager,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: ConfigManager::new(),
            session: SessionManager::new(),
        }
    }
}
