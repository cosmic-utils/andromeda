#[derive(Debug, Clone)]
pub struct Error {
    pub(crate) description: String,
    pub(crate) recoverable: bool,
}

impl Error {
    pub fn new(desc: impl Into<String>, recoverable: bool) -> Self {
        Self {
            description: desc.into(),
            recoverable,
        }
    }
}

impl From<udisks2::Error> for Error {
    fn from(error: udisks2::Error) -> Self {
        Self::new(error.to_string(), false)
    }
}
