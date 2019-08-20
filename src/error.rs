use std::io;
use std::path::PathBuf;

#[must_use]
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Generic IO error
    Io(io::Error),
    /// Invalid argument
    Argument(Argument),
    /// Unknown target ABI
    UnknownTarget,
    /// Nasm failed to execute
    Nasm,
    /// Linker failed to execute
    Linker,
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

#[derive(Debug)]
pub enum Argument {
    /// Path: Required file, got directory
    FileRequired(PathBuf),
}
