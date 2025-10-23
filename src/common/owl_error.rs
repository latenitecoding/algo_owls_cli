use std::fmt;

pub type Result<T> = std::result::Result<T, OwlError>;

#[derive(Debug)]
pub enum OwlError {
    CommandNotFound(String),
    FileError(String, String),
    LlmError(String, String),
    NetworkError(String, String),
    ProcessError(String, String),
    TestFailure(String),
    TomlError(String, String),
    TuiError(String, String),
    Unsupported(String),
    UriError(String, String),
}

macro_rules! check_info {
    ($err_info:expr) => {
        if $err_info.is_empty() {
            "N/A"
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
            OwlError::TuiError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
            OwlError::Unsupported(expr) => write!(f, "{}", expr),
            OwlError::UriError(expr, err_info) => {
                write!(f, "{} (info: {})", expr, check_info!(err_info))
            }
        }
    }
}
