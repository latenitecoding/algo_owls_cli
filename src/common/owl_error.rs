use std::fmt;

#[derive(Debug)]
pub enum OwlError {
    CommandNotFound(String),
    FileError(String, String),
    LlmError(String, String),
    NetworkError(String, String),
    ProcessError(String, String),
    TestFailure(String),
    TomlError(String, String),
    Unsupported(String),
    UriError(String, String),
}

macro_rules! check_info {
    ($err_info:expr) => {
        if $err_info.is_empty() {
            "None"
        } else {
            $err_info
        }
    };
}

impl fmt::Display for OwlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwlError::CommandNotFound(expr) => write!(f, "{}", expr),
            OwlError::FileError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
            OwlError::LlmError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
            OwlError::NetworkError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
            OwlError::ProcessError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
            OwlError::TestFailure(expr) => write!(f, "{}", expr),
            OwlError::TomlError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
            OwlError::Unsupported(expr) => write!(f, "{}", expr),
            OwlError::UriError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
        }
    }
}
