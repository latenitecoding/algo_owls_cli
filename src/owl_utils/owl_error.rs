use std::fmt;

#[derive(Debug)]
pub enum OwlError {
    CommandNotFound(String, String),
    FileError(String, String),
    ManifestError(String, String),
    NetworkError(String, String),
    ProcessError(String, String),
    TestFailure(String),
    TimeError(String, String),
    UnrecognizedChars(String),
    UnsupportedLanguage(String),
}

impl fmt::Display for OwlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwlError::CommandNotFound(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::FileError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::ManifestError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::NetworkError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::ProcessError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::TestFailure(e) => write!(f, "{}", e),
            OwlError::TimeError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::UnrecognizedChars(e) => write!(f, "{}", e),
            OwlError::UnsupportedLanguage(e) => write!(f, "{}", e),
        }
    }
}

#[macro_export]
macro_rules! check_manifest {
    ($expr:expr, $name:expr) => {
        $expr
            .get($name)
            .map(|entry| entry.as_value())
            .flatten()
            .map(|entry| entry.as_str())
            .flatten()
            .map(|entry| entry.to_string())
            .ok_or(no_entry_found!("check_manifest", $name))
    };
}

pub(crate) use check_manifest;

#[macro_export]
macro_rules! check_item {
    ($expr:expr, $name:expr) => {
        $expr
            .as_value()
            .map(|entry| entry.as_str())
            .flatten()
            .map(|entry| entry.to_string())
            .ok_or(no_entry_found!("check_item", $name))
    };
}

pub(crate) use check_item;

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
    ($tag:expr, $expr:expr) => {
        OwlError::CommandNotFound($tag.to_string(), format!("command not found: {}", $expr))
    };
}

#[macro_export]
macro_rules! file_error {
    ($tag:expr, $expr:expr) => {
        OwlError::FileError($tag.to_string(), format!("{}", $expr.to_string()))
    };
}

pub(crate) use file_error;

#[macro_export]
macro_rules! file_not_found {
    ($tag:expr, $expr:expr) => {
        OwlError::FileError(
            $tag.to_string(),
            format!("'{}': No such file or directory (os error 2)", $expr),
        )
    };
}

pub(crate) use file_not_found;

#[macro_export]
macro_rules! manifest_error {
    ($tag:expr, $expr:expr) => {
        OwlError::ManifestError($tag.to_string(), $expr.to_string())
    };
}

pub(crate) use manifest_error;

#[macro_export]
macro_rules! net_error {
    ($tag:expr, $expr:expr) => {
        OwlError::NetworkError($tag.to_string(), $expr.to_string())
    };
}

pub(crate) use net_error;

#[macro_export]
macro_rules! no_entry_found {
    ($tag:expr, $expr:expr) => {
        OwlError::ManifestError(
            $tag.to_string(),
            format!("'{}': No such entry in manifest", $expr),
        )
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
macro_rules! process_error {
    ($tag:expr, $expr:expr) => {
        OwlError::ProcessError($tag.to_string(), $expr.to_string())
    };
}

pub(crate) use process_error;

#[macro_export]
macro_rules! test_failure {
    ($expr:expr) => {
        OwlError::TestFailure($expr.to_string())
    };
}

#[macro_export]
macro_rules! time_error {
    ($tag:expr, $expr:expr) => {
        OwlError::TimeError($tag.to_string(), $expr.to_string())
    };
}

pub(crate) use time_error;
