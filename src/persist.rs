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
        // URL 이력 등 민감 정보가 들어갈 수 있으므로 디렉터리를 소유자 전용(0700)으로.
        restrict_to_owner(parent, 0o700);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = write_private(&path, json.as_bytes());
    }
}

/// 파일을 소유자만 읽고 쓸 수 있도록(0600) 생성/갱신한다.
/// URL 쿼리 파라미터에 API 키·토큰이 포함될 수 있어 같은 시스템의
/// 다른 사용자가 읽지 못하게 막는다. (unix 외 플랫폼은 일반 쓰기)
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
        // 기존 파일이 더 느슨한 권한(0644)으로 이미 존재하던 경우를 대비해 명시적으로 재설정.
        restrict_to_owner(path, 0o600);
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes)
    }
}

/// unix에서 경로 권한을 소유자 전용 비트로 제한한다. (다른 플랫폼은 no-op)
fn restrict_to_owner(_path: &std::path::Path, _mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(_path, std::fs::Permissions::from_mode(_mode));
    }
}
