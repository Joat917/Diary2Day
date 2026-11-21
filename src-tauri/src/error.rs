use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: String,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "{}: {} ({detail})", self.code, self.msg),
            None => write!(f, "{}: {}", self.code, self.msg),
        }
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn new(code: &str, msg: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            msg: msg.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new("internal", msg)
    }

    pub fn io(err: impl std::fmt::Display) -> Self {
        Self::new("io", "文件读写失败").with_detail(err.to_string())
    }

    pub fn sqlite(err: impl std::fmt::Display) -> Self {
        Self::new("sqlite", "数据库操作失败").with_detail(err.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::sqlite(value)
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::internal("数据序列化失败").with_detail(value.to_string())
    }
}

impl From<chrono::ParseError> for AppError {
    fn from(value: chrono::ParseError) -> Self {
        Self::new("bad_time", "时间格式无法解析").with_detail(value.to_string())
    }
}
