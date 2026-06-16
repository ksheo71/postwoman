//! 테마·URL 이력 등 앱 상태를 디스크에 저장/로드한다.
//! macOS의 Application Support 디렉터리에 JSON으로 보관한다.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub theme: Theme,
    /// 최근 사용한 URL (가장 최근이 맨 앞).
    pub url_history: Vec<String>,
}

fn state_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support/Postwoman")
            .join("state.json"),
    )
}

pub fn load() -> AppState {
    state_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(state: &AppState) {
    let Some(path) = state_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, json);
    }
}
