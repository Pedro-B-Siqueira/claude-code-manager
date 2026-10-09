use std::path::PathBuf;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("variável HOME ausente no ambiente")]
    MissingHome,
    #[error("escrita proibida em {0}: área somente leitura")]
    ForbiddenWrite(PathBuf),
    #[error("falha de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("falha no banco local: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("dados inválidos: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::MissingHome => "missingHome",
            Self::ForbiddenWrite(_) => "forbiddenWrite",
            Self::Io(_) => "io",
            Self::Database(_) => "database",
            Self::Serialization(_) => "serialization",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut payload = serializer.serialize_struct("AppError", 2)?;
        payload.serialize_field("kind", self.kind())?;
        payload.serialize_field("message", &self.to_string())?;
        payload.end()
    }
}
