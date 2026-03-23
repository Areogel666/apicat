/// Tauri command 统一错误类型——实现 Serialize 才能跨 IPC 传到前端
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Custom(String),
}

// Tauri 要求 command 错误类型实现 serde::Serialize
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type CmdResult<T> = Result<T, AppError>;

/// 将 sqlx 错误转为用户友好提示：
/// - UNIQUE constraint failed: api_requests.name → "接口名「x」在当前文件夹中已存在"
/// - 其他错误 → 原始 AppError::Db
pub fn map_unique_name_error(e: sqlx::Error, name: &str) -> AppError {
    let msg = e.to_string();
    if msg.contains("UNIQUE constraint failed") && msg.contains("api_requests") {
        AppError::Custom(format!("接口名「{}」在当前文件夹中已存在", name))
    } else {
        AppError::Db(e)
    }
}
