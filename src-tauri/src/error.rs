use serde::Serialize;

// 轻量应用错误（IPC 返回给前端）
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub message: String,
}

impl AppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(e.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
