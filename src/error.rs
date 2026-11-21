//! Error types for the PowerPoint document engine.
#[derive(Debug)]
pub enum FPptError {
    /// I/O failure when reading or writing files.
    Io(std::io::Error),
    /// The archive could not be opened or a required entry is missing.
    Zip(zip::result::ZipError),
    /// The XML content is malformed or does not match the expected schema.
    Xml(String),
    /// The requested operation is not valid for the current document state.
    Invalid(String),
}
impl std::fmt::Display for FPptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FPptError::Io(e) => write!(f, "io error: {}", e),
            FPptError::Zip(e) => write!(f, "zip error: {}", e),
            FPptError::Xml(e) => write!(f, "xml error: {}", e),
            FPptError::Invalid(e) => write!(f, "invalid operation: {}", e),
        }
    }
}
impl std::error::Error for FPptError {}
impl From<std::io::Error> for FPptError {
    fn from(e: std::io::Error) -> Self {
        FPptError::Io(e)
    }
}
impl From<zip::result::ZipError> for FPptError {
    fn from(e: zip::result::ZipError) -> Self {
        FPptError::Zip(e)
    }
}
pub type FPptResult<T> = Result<T, FPptError>;
