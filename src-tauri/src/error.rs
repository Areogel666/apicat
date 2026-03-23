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
