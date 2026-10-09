//! Development-only smoke scenario, driven by env vars, used to exercise UI → PTY → xterm without
//! clicking through the window. Release builds always report no scenario.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevScenario {
    pub name: String,
    pub cwd: String,
}

#[tauri::command]
pub fn dev_scenario() -> Option<DevScenario> {
    if cfg!(debug_assertions) {
        let name = std::env::var("CCM_E2E").ok().filter(|name| !name.is_empty())?;
        let cwd = std::env::var("CCM_E2E_CWD").ok().filter(|cwd| !cwd.is_empty())?;
        Some(DevScenario { name, cwd })
    } else {
        None
    }
}
