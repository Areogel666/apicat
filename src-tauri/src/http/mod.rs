pub mod client;
pub mod variable;

/// 全局 HTTP 客户端，Tauri setup 时创建一次，通过 State<HttpClient> 注入 Command
/// 复用连接池和 TLS session cache，避免每次请求重建开销
pub struct HttpClient(pub reqwest::Client);
