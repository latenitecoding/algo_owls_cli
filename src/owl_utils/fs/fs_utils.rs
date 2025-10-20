use crate::common::OwlError;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, copy};
use std::path::{Path, PathBuf};
use url::Url;
use zip::ZipArchive;

pub fn copy_file(src: &Path, dst: &Path) -> Result<(), OwlError> {
    let mut src_file = OpenOptions::new().read(true).open(src).map_err(|e| {
        OwlError::FileError(
            format!("could not open on copy '{}'", src.to_string_lossy()),
            e.to_string(),
        )
    })?;
    let mut dst_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dst)
        .map_err(|e| {
            OwlError::FileError(
                format!("could not truncate on copy '{}'", dst.to_string_lossy()),
                e.to_string(),
            )
        })?;

    copy(&mut src_file, &mut dst_file).map_err(|e| {
        OwlError::FileError(
            format!(
                "could not copy '{}' -> '{}'",
                src.to_string_lossy(),
                dst.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn dir_tree(root_dir: &Path) -> Result<Vec<PathBuf>, OwlError> {
    if !root_dir.exists() {
        return Err(OwlError::FileError(
            format!("'{}': no such directory", root_dir.to_string_lossy()),
            "".into(),
        ));
    }

    if !root_dir.is_dir() {
        return Err(OwlError::FileError(
            format!("'{}': is not a directory", root_dir.to_string_lossy()),
            "".into(),
        ));
    }

    let mut files: Vec<PathBuf> = Vec::new();

    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root_dir.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(&dir).map_err(|e| {
            OwlError::FileError(
                format!("could not read dir '{}'", dir.to_string_lossy()),
                e.to_string(),
            )
        })? {
            let path = entry
                .map_err(|e| {
                    OwlError::FileError(
                        format!("could not read entry in dir '{}'", dir.to_string_lossy()),
                        e.to_string(),
                    )
                })?
                .path();

            if path.is_dir() {
                queue.push_back(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}

pub async fn download_archive(
    url: &Url,
    tmp_archive: &Path,
    out_dir: &Path,
) -> Result<(), OwlError> {
    download_file(url, tmp_archive).await?;
    extract_archive(tmp_archive, out_dir)?;
    remove_path(tmp_archive)
}

pub async fn download_file(url: &Url, out: &Path) -> Result<(), OwlError> {
    let resp = reqwest::get(url.as_str())
        .await
        .map_err(|e| OwlError::NetworkError(format!("could not request '{}'", url), e.to_string()))?
        .bytes()
        .await
        .map_err(|e| {
            OwlError::NetworkError(format!("could not read response '{}'", url), e.to_string())
        })?;

    let mut cursor = Cursor::new(resp);

    let mut out_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out)
        .map_err(|e| {
            OwlError::FileError(
                format!(
                    "could not truncate download file '{}'",
                    out.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;

    copy(&mut cursor, &mut out_file).map_err(|e| {
        OwlError::FileError(
            format!("could not copy '{}' -> '{}'", url, out.to_string_lossy()),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn ensure_path_from_home(dirs: &[&str], file_str: Option<&str>) -> Result<PathBuf, OwlError> {
    let mut path = dirs::home_dir().ok_or(OwlError::FileError(
        "could not find home dir".into(),
        "".into(),
    ))?;
    for dir in dirs {
        path.push(dir);
    }

    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| {
            OwlError::FileError(
                format!("could not create path '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })?;
    }

    if let Some(filename) = file_str {
        path.push(filename);
    }

    Ok(path)
}

pub fn extract_archive(archive_file: &Path, out_dir: &Path) -> Result<(), OwlError> {
    let zip_file = OpenOptions::new()
        .read(true)
        .open(archive_file)
        .map_err(|e| {
            OwlError::FileError(
                format!(
                    "could not open archive '{}'",
                    archive_file.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;

    let mut zip_archive = ZipArchive::new(zip_file).map_err(|e| {
        OwlError::FileError(
            format!(
                "could not parse archive '{}'",
                archive_file.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    if !out_dir.exists() {
        fs::create_dir_all(out_dir).map_err(|e| {
            OwlError::FileError(
                format!("could not create path '{}'", out_dir.to_string_lossy()),
                e.to_string(),
            )
        })?;
    }

    zip_archive.extract(out_dir).map_err(|e| {
        OwlError::FileError(
            format!(
                "could not extract '{}' -> '{}'",
                archive_file.to_string_lossy(),
                out_dir.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn find_by_ext(root_dir: &Path, target_ext: &str) -> Result<Vec<PathBuf>, OwlError> {
    dir_tree(root_dir)
        .map(|files| {
            files
                .into_iter()
                .filter(|file| {
                    if let Some(ext) = file.extension()
                        && ext == target_ext
                    {
                        true
                    } else {
                        false
                    }
                })
                .collect::<Vec<PathBuf>>()
        })
        .map_or_else(Err, |files| {
            if files.is_empty() {
                Err(OwlError::FileError(
                    format!(
                        "no matches in '{}' matching '{}'",
                        root_dir.to_string_lossy(),
                        target_ext
                    ),
                    "".into(),
                ))
            } else {
                Ok(files)
            }
        })
}

pub fn find_by_stem_and_ext(
    root_dir: &Path,
    target_stem: &str,
    target_ext: &str,
) -> Result<PathBuf, OwlError> {
    dir_tree(root_dir)
        .map(|files| {
            files.into_iter().find(|file| {
                if let Some(stem) = file.file_stem()
                    && stem == target_stem
                    && let Some(ext) = file.extension()
                    && ext == target_ext
                {
                    true
                } else {
                    false
                }
            })
        })
        .map_or_else(Err, |file| match file {
            Some(target_file) => Ok(target_file),
            None => Err(OwlError::FileError(
                format!(
                    "no matches in '{}' matching '{}.{}'",
                    root_dir.to_string_lossy(),
                    target_stem,
                    target_ext
                ),
                "".into(),
            )),
        })
}

pub fn remove_path(path: &Path) -> Result<(), OwlError> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| {
            OwlError::FileError(
                format!(
                    "could not remove-recursively dir '{}'",
                    path.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;
    } else if path.is_file() {
        fs::remove_file(path).map_err(|e| {
            OwlError::FileError(
                format!("could not remove file '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })?;
    }

    Ok(())
}
