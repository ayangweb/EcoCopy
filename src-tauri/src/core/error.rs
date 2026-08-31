use serde::{ser::SerializeStruct, Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Clipboard(String),

    #[error("{0}")]
    Ai(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            AppError::Clipboard(_) => "Clipboard",
            AppError::Ai(_) => "Ai",
            AppError::Other(_) => "Other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
