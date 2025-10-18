use std::fmt;

#[derive(Debug)]
pub enum OwlError {
    CommandNotFound(String, String),
    FileError(String, String),
    ManifestError(String, String),
    NetworkError(String, String),
    ProcessError(String, String),
    TestFailure(String),
    UnrecognizedChars(String, String),
    UnsupportedLanguage(String),
}

impl fmt::Display for OwlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwlError::CommandNotFound(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::FileError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::ManifestError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::NetworkError(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::ProcessError(tag, e) => write!(f, "[{}; failed] {}", tag, e),
            OwlError::TestFailure(e) => write!(f, "{}", e),
            OwlError::UnrecognizedChars(tag, e) => write!(f, "[{}] {}", tag, e),
            OwlError::UnsupportedLanguage(e) => write!(f, "{}", e),
        }
    }
}

#[macro_export]
macro_rules! bad_chars {
    ($tag:expr) => {
        OwlError::UnrecognizedChars(
            $tag.to_string(),
            "buffer has non UTF-8 chars or is empty".to_string(),
        )
    };
}

pub(crate) use bad_chars;

#[macro_export]
macro_rules! check_file_ext {
    ($path:expr) => {
        $path
            .extension()
            .and_then(OsStr::to_str)
            .ok_or(match $path.to_str() {
                Some(path_str) => {
                    OwlError::FileError(path_str.to_string(), "file has no extension".to_string())
                }
                None => OwlError::FileError(
                    "check_file_ext".to_string(),
                    "path is empty or unrecognizable".to_string(),
                ),
            })
    };
}

#[macro_export]
macro_rules! check_file_stem {
    ($path:expr) => {
        $path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or(match $path.to_str() {
                Some(path_str) => {
                    OwlError::FileError(path_str.to_string(), "file has no stem".to_string())
                }
                None => OwlError::FileError(
                    "check_file_stem".to_string(),
                    "path is empty or unrecognizable".to_string(),
                ),
            })
    };
}

pub(crate) use check_file_stem;

#[macro_export]
macro_rules! check_item {
    ($item:expr, $name:expr) => {
        $item
            .as_value()
            .map(|entry| entry.as_str())
            .flatten()
            .map(|entry| entry.to_string())
            .ok_or(OwlError::ManifestError(
                $name.to_string(),
                format!("'{}': No such entry in manifest", $name),
            ))
    };
}

pub(crate) use check_item;

#[macro_export]
macro_rules! check_manifest {
    ($doc:expr, $name:expr) => {
        $doc.get($name)
            .map(|entry| entry.as_value())
            .flatten()
            .map(|entry| entry.as_str())
            .flatten()
            .map(|entry| entry.to_string())
            .ok_or(OwlError::ManifestError(
                $name.to_string(),
                format!("'{}': No such entry in manifest", $name),
            ))
    };
}

pub(crate) use check_manifest;

#[macro_export]
macro_rules! check_parent {
    ($path:expr) => {
        $path
            .parent()
            .ok_or(match $path.to_str() {
                Some(path_str) => {
                    OwlError::FileError(path_str.to_string(), "file has no parent".to_string())
                }
                None => OwlError::FileError(
                    "check_parent".to_string(),
                    "path is empty or unrecognizable".to_string(),
                ),
            })
            .map(Path::to_path_buf)
    };
}

pub(crate) use check_parent;

#[macro_export]
macro_rules! check_path {
    ($path:expr) => {
        $path.to_str().ok_or(OwlError::FileError(
            "check_path".to_string(),
            "path is empty or unrecognizable".to_string(),
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
    ($name:expr) => {
        OwlError::ManifestError(
            $name.to_string(),
            format!("'{}': No such entry in manifest", $name),
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
