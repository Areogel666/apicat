// 1.0.5：Localhost HTTP Bridge —— 把 IPC 能力暴露给外部 AI 技能
// 安全：绑 127.0.0.1 + 启动生成随机 token 写 bridge.json；请求头 Authorization: Bearer <token>
// 协议：REST，前缀 /api/v1/，路径对齐 IPC command 名；GET=只读（query 参数），POST=写（JSON body）
// 写操作后广播 bridge-data-changed 事件，前端各 store 自行 reload

pub mod server;
pub mod token;

use std::path::PathBuf;
use tauri::Manager;

/// bridge.json 的内容
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeInfo {
    pub port: u16,
    pub token: String,
    pub enabled: bool,
}

/// bridge.json 路径：%APPDATA%/com.apicat.app/bridge.json
pub fn bridge_file_path(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(app.path().app_data_dir()?.join("bridge.json"))
}

/// 从 app-settings.json 读 bridge 开关（默认 true）
/// 用简单文件读取，避免依赖 tauri-plugin-store 的异步 API
pub fn read_bridge_enabled(app: &tauri::App) -> bool {
    let path = match app.path().app_data_dir() {
        Ok(d) => d.join("app-settings.json"),
        Err(_) => return true,
    };
    let Ok(raw) = std::fs::read_to_string(&path) else { return true };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { return true };
    // 支持两种存放位置：顶层 bridgeEnabled 或 settings 对象内
    v.get("bridgeEnabled")
        .or_else(|| v.get("settings").and_then(|s| s.get("bridgeEnabled")))
        .and_then(|b| b.as_bool())
        .unwrap_or(true)
}
