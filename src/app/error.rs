#[derive(Debug, Clone)]
pub struct Error {
    pub(crate) description: String,
    pub(crate) recoverable: bool,
}

impl Error {
    pub fn from_err(err: Box<dyn std::error::Error>, recoverable: bool) -> Self {
        Self {
            description: err.to_string(),
            recoverable,
        }
    }

    pub fn new(desc: impl Into<String>, recoverable: bool) -> Self {
        Self {
            description: desc.into(),
            recoverable,
        }
    }
}
