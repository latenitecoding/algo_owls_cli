use crate::common::{OwlError, Result};
use crate::owl_utils::{Uri, fs_utils, toml_utils};
use crate::{MANIFEST, OWL_DIR, PROMPT_DIR, STASH_DIR, TMP_ARCHIVE, TOML_TEMPLATE};
use std::path::Path;
use toml_edit::{DocumentMut, value};

pub async fn add_extension(ext_name: &str, ext_uri: &Uri, and_fetch: bool) -> Result<()> {
    let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))?;
    let prompt_dir = fs_utils::ensure_path_from_home(&[OWL_DIR, STASH_DIR, PROMPT_DIR], None)?;

    let mut manifest_doc = if manifest_path.exists() {
        toml_utils::read_toml(&manifest_path)?
    } else {
        TOML_TEMPLATE.parse::<DocumentMut>().map_err(|e| {
            OwlError::TomlError("could not parse TOML template".into(), e.to_string())
        })?
    };

    let (uri_str, ext_doc) = match ext_uri {
        Uri::Local(path) => {
            let uri_str = path
                .to_str()
                .ok_or(OwlError::UriError("invalid URI".into(), "".into()))?;
            (uri_str, toml_utils::read_toml(path)?)
        }
        Uri::Remote(url) => (url.as_str(), toml_utils::request_toml(url).await?),
    };

    manifest_doc["extensions"][ext_name] = value(uri_str);

    let some_tmp_archive = if and_fetch {
        Some(Path::new(TMP_ARCHIVE))
    } else {
        None
    };

    toml_utils::commit_extension(
        &manifest_path,
        &prompt_dir,
        ext_name,
        ext_uri,
        &ext_doc,
        &mut manifest_doc,
        some_tmp_archive,
    )
    .await
}

pub async fn add_quest(quest_name: &str, uri: &Uri, and_fetch: bool) -> Result<()> {
    let manifest_path = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(MANIFEST))?;

    let mut manifest_doc = if manifest_path.exists() {
        toml_utils::read_toml(&manifest_path)?
    } else {
        TOML_TEMPLATE.parse::<DocumentMut>().map_err(|e| {
            OwlError::TomlError("could not parse TOML template".into(), e.to_string())
        })?
    };

    let uri_str = match uri {
        Uri::Local(path) => path
            .to_str()
            .ok_or(OwlError::UriError("invalid URI".into(), "".into()))?,
        Uri::Remote(url) => url.as_str(),
    };

    manifest_doc["personal"][quest_name] = value(uri_str);

    toml_utils::write_manifest(&manifest_doc, &manifest_path)?;

    if and_fetch {
        let quest_dir = fs_utils::ensure_path_from_home(&[OWL_DIR], Some(quest_name))?;

        match uri {
            Uri::Local(path) => fs_utils::extract_archive(path, &quest_dir)?,
            Uri::Remote(url) => {
                fs_utils::download_archive(url, Path::new(TMP_ARCHIVE), &quest_dir).await?
            }
        }
    }

    Ok(())
}
