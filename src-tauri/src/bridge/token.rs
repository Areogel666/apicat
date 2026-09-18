// token 生成与 bridge.json 读写

use super::BridgeInfo;
use std::path::Path;

/// 生成随机 token（32 字节 → 64 hex 字符）
pub fn generate_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // 轻量随机：时间戳 + 地址熵 + 简单 xorshift，避免引入 rand 依赖
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let addr = &nanos as *const u128 as u64;
    let mut state = nanos as u64 ^ addr ^ (nanos >> 64) as u64;
    let mut out = String::with_capacity(64);
    for _ in 0..4 {
        // xorshift64
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        out.push_str(&format!("{:016x}", state));
    }
    out
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
