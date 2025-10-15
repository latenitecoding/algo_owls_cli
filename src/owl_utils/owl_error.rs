use std::fmt;

#[derive(Debug)]
pub enum OwlError {
    CommandNotFound(String),
    FileError(String),
    ManifestError(String),
    NetworkError(String),
    ProgramError(String),
    TestFailure(String),
    UnrecognizedChars(String),
    UnsupportedLanguage(String),
}

impl fmt::Display for OwlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwlError::CommandNotFound(e) => write!(f, "{}", e),
            OwlError::FileError(e) => write!(f, "{}", e),
            OwlError::ManifestError(e) => write!(f, "{}", e),
            OwlError::NetworkError(e) => write!(f, "{}", e),
            OwlError::ProgramError(e) => write!(f, "{}", e),
            OwlError::TestFailure(e) => write!(f, "{}", e),
            OwlError::UnrecognizedChars(e) => write!(f, "{}", e),
            OwlError::UnsupportedLanguage(e) => write!(f, "{}", e),
        }
    }
}

#[macro_export]
macro_rules! check_path {
    ($expr:expr) => {
        $expr.to_str().ok_or(OwlError::UnrecognizedChars(
            $expr.to_string_lossy().into_owned(),
        ))
    };
}

pub(crate) use check_path;

#[macro_export]
macro_rules! command_not_found {
    ($expr:expr) => {
        OwlError::CommandNotFound(format!("command not found: {}", $expr))
    };
}

#[macro_export]
macro_rules! file_error {
    ($text:literal) => {
        OwlError::FileError($text.to_string())
    };
    ($expr:expr) => {
        OwlError::FileError($expr.to_string())
    };
}

pub(crate) use file_error;

#[macro_export]
macro_rules! file_not_found {
    ($expr:expr) => {
        OwlError::FileError(format!(
            "'{}': No such file or directory (os error 2)",
            $expr
        ))
    };
}

pub(crate) use file_not_found;

#[macro_export]
macro_rules! program_error {
    ($expr:expr) => {
        OwlError::ProgramError($expr.to_string())
    };
}

pub(crate) use program_error;

#[macro_export]
macro_rules! net_error {
    ($text:literal) => {
        OwlError::NetworkError($text.to_string())
    };
    ($expr:expr) => {
        OwlError::NetworkError($expr.to_string())
    };
}

pub(crate) use net_error;

#[macro_export]
macro_rules! no_entry_found {
    ($expr:expr) => {
        OwlError::ManifestError(format!("'{}': No such entry in manifest", $expr))
    };
}

pub(crate) use no_entry_found;

#[macro_export]
macro_rules! not_supported {
    ($expr:expr) => {
        OwlError::UnsupportedLanguage(format!("Language not supported: {}", $expr))
    };
}

pub(crate) use not_supported;

#[macro_export]
macro_rules! test_failure {
    ($text:literal) => {
        OwlError::TestFailure($text.to_string())
    };
    ($expr:expr) => {
        OwlError::TestFailure($expr.to_string())
    };
}
