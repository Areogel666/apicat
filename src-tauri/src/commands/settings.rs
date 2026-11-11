use tauri::State;

use crate::http::{CompressionSettings, HttpClient};

/// 「响应解压」开关即时生效：前端直接传当前值，避免 plugin-store 落盘防抖
/// 导致 Rust 读到旧 app-settings.json 的竞态。
#[tauri::command]
pub fn apply_network_settings(
    http: State<'_, HttpClient>,
    enabled: bool,
    algorithms: Vec<String>,
) -> Result<(), String> {
    http.replace(CompressionSettings::from_parts(enabled, &algorithms));
    Ok(())
}
