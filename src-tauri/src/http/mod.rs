pub mod client;
pub mod variable;

use std::sync::{Arc, RwLock};
use std::time::Duration;

/// 响应解压配置。默认全关（Accept-Encoding 不发）；开关打开后按勾选算法解压。
/// 配置真源是 app-settings.json 的 `network.decompressEnabled` /
/// `network.decompressAlgorithms`，前端「开关设置」弹窗写入。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompressionSettings {
    pub gzip: bool,
    pub deflate: bool,
    pub brotli: bool,
    pub zstd: bool,
}

impl CompressionSettings {
    /// 实际会发出的 Accept-Encoding 值；全关返回 None。
    /// 顺序与 reqwest/tower-http 自动拼接一致：zstd,gzip,deflate,br
    pub fn accept_encoding(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.zstd {
            parts.push("zstd");
        }
        if self.gzip {
            parts.push("gzip");
        }
        if self.deflate {
            parts.push("deflate");
        }
        if self.brotli {
            parts.push("br");
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(","))
        }
    }

    /// 从算法列表构造；enabled=false 时一律全关。
    pub fn from_parts(enabled: bool, algorithms: &[String]) -> Self {
        if !enabled {
            return Self::default();
        }
        let has = |names: &[&str]| algorithms.iter().any(|s| names.contains(&s.as_str()));
        Self {
            gzip: has(&["gzip"]),
            deflate: has(&["deflate"]),
            brotli: has(&["br", "brotli"]),
            zstd: has(&["zstd"]),
        }
    }

    /// 从 app-settings.json 的 JSON 对象解析（兼容顶层 / `settings` 嵌套）。
    pub fn from_json(v: &serde_json::Value) -> Self {
        let settings = v.get("settings").unwrap_or(v);
        let enabled = settings
            .get("network.decompressEnabled")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        if !enabled {
            return Self::default();
        }
        let Some(arr) = settings
            .get("network.decompressAlgorithms")
            .and_then(|a| a.as_array())
        else {
            // 开关开了但算法数组缺失：按前端默认值兜底
            return Self { gzip: true, deflate: true, brotli: true, zstd: false };
        };
        let algorithms: Vec<String> = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
        Self::from_parts(true, &algorithms)
    }
}

/// 读当前压缩配置；文件缺失/解析失败一律回落「全关」。
pub fn read_compression_settings(app: &tauri::AppHandle) -> CompressionSettings {
    use tauri::Manager;
    let Ok(dir) = app.path().app_data_dir() else {
        return CompressionSettings::default();
    };
    let Ok(raw) = std::fs::read_to_string(dir.join("app-settings.json")) else {
        return CompressionSettings::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return CompressionSettings::default();
    };
    CompressionSettings::from_json(&v)
}

/// 构造 reqwest client 的公共参数。主 client 与压测 client 必须共用，
/// 保证两边发出的 UA / Accept-Encoding 一致。
pub fn client_builder(c: CompressionSettings) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(30))
        // 默认 UA：可被用户手动 User-Agent 行覆盖（请求级 header 优先）
        .user_agent(concat!("ApiCat/", env!("CARGO_PKG_VERSION")))
        .gzip(c.gzip)
        .deflate(c.deflate)
        .brotli(c.brotli)
        .zstd(c.zstd)
}

pub fn build_http_client(c: CompressionSettings) -> reqwest::Client {
    client_builder(c)
        .build()
        .expect("failed to build HTTP client")
}

/// 全局 HTTP 客户端，Tauri setup 时创建一次，通过 State<HttpClient> 注入 Command
/// 复用连接池和 TLS session cache，避免每次请求重建开销。
/// 内部用 RwLock 支持「响应解压」开关即时生效：设置变化时重建并替换。
#[derive(Clone)]
pub struct HttpClient(Arc<RwLock<reqwest::Client>>);

impl HttpClient {
    pub fn new(c: CompressionSettings) -> Self {
        Self(Arc::new(RwLock::new(build_http_client(c))))
    }

    /// 取一个 client 克隆（reqwest::Client 内部是 Arc，克隆很便宜）。
    pub fn client(&self) -> reqwest::Client {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn replace(&self, c: CompressionSettings) {
        *self.0.write().unwrap_or_else(|e| e.into_inner()) = build_http_client(c);
    }
}

#[cfg(test)]
mod tests {
    use super::CompressionSettings;

    fn algs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn disabled_sends_no_accept_encoding() {
        let c = CompressionSettings::from_parts(false, &algs(&["gzip", "zstd"]));
        assert_eq!(c.accept_encoding(), None);
    }

    #[test]
    fn default_algorithms_match_reqwest_order() {
        let c = CompressionSettings::from_parts(true, &algs(&["gzip", "deflate", "br"]));
        assert_eq!(c.accept_encoding().as_deref(), Some("gzip,deflate,br"));
    }

    #[test]
    fn all_algorithms_put_zstd_first() {
        let c = CompressionSettings::from_parts(true, &algs(&["gzip", "deflate", "br", "zstd"]));
        assert_eq!(c.accept_encoding().as_deref(), Some("zstd,gzip,deflate,br"));
    }

    #[test]
    fn empty_or_unknown_algorithms_send_nothing() {
        assert_eq!(
            CompressionSettings::from_parts(true, &[]).accept_encoding(),
            None
        );
        assert_eq!(
            CompressionSettings::from_parts(true, &algs(&["identity"])).accept_encoding(),
            None
        );
    }
}
