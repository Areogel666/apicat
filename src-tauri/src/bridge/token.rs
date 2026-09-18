// token 生成与 bridge.json 读写

use super::BridgeInfo;
use std::path::Path;

/// 生成随机 token（UUID v4，去掉连字符 → 32 hex 字符）
pub fn generate_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// 读 bridge.json；不存在或损坏则返回 None
pub fn read_bridge_info(path: &Path) -> Option<BridgeInfo> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 写 bridge.json
pub fn write_bridge_info(path: &Path, info: &BridgeInfo) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(info)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}
