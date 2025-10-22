use crate::common::OwlError;
use std::path::PathBuf;
use url::Url;

#[derive(Clone, Debug)]
pub enum Uri {
    Local(PathBuf),
    Remote(Url),
}

impl TryFrom<&str> for Uri {
    type Error = OwlError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.is_empty() {
            Err(OwlError::UriError(
                "Failed to parse URI".into(),
                "is empty string".into(),
            ))
        } else if let Ok(url) = Url::parse(s) {
            Ok(Uri::Remote(url))
        } else {
            Ok(Uri::Local(PathBuf::from(s)))
        }
    }
}
