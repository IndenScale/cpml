use thiserror::Error;

#[derive(Debug, Error)]
pub enum CpmlError {
    #[error("YAML parse error: {0}")]
    ParseError(#[from] serde_yaml::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Date parse error: {0}")]
    DateParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Reference not found: {0}")]
    ReferenceError(String),

    #[error("Duplicate ID: {0}")]
    DuplicateIdError(String),

    #[error("Type mismatch: {0}")]
    TypeMismatchError(String),
}
