use crate::common::{OwlError, Result};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, copy};
use std::path::{Path, PathBuf};
use url::Url;
use zip::ZipArchive;

pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let mut src_file = OpenOptions::new().read(true).open(src).map_err(|e| {
        OwlError::FileError(
            format!("Failed to open '{}' for reading", src.to_string_lossy()),
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
                format!("Failed to truncate '{}' for writing", dst.to_string_lossy()),
                e.to_string(),
            )
        })?;

    copy(&mut src_file, &mut dst_file).map_err(|e| {
        OwlError::FileError(
            format!(
                "Failed to copy '{}' into '{}'",
                src.to_string_lossy(),
                dst.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn dir_tree(root_dir: &Path) -> Result<Vec<PathBuf>> {
    if !root_dir.exists() {
        return Err(OwlError::FileError(
            format!("Failed to access dir '{}'", root_dir.to_string_lossy()),
            "no such directory <os error 2>".into(),
        ));
    }

    if !root_dir.is_dir() {
        return Err(OwlError::FileError(
            format!(
                "Failed to read entries in dir '{}'",
                root_dir.to_string_lossy()
            ),
            "is file, not directory".into(),
        ));
    }

    let mut files: Vec<PathBuf> = Vec::new();

    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root_dir.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(&dir).map_err(|e| {
            OwlError::FileError(
                format!("Failed to read dir '{}'", dir.to_string_lossy()),
                e.to_string(),
            )
        })? {
            let path = entry
                .map_err(|e| {
                    OwlError::FileError(
                        format!(
                            "Failed to determine path of dir entry '{}'",
                            dir.to_string_lossy()
                        ),
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

pub async fn download_archive(url: &Url, tmp_archive: &Path, out_dir: &Path) -> Result<()> {
    download_file(url, tmp_archive).await?;
    extract_archive(tmp_archive, out_dir)?;
    remove_path(tmp_archive)
}

pub async fn download_file(url: &Url, out: &Path) -> Result<()> {
    let resp = reqwest::get(url.as_str())
        .await
        .map_err(|e| OwlError::NetworkError(format!("Failed to request '{}'", url), e.to_string()))?
        .bytes()
        .await
        .map_err(|e| {
            OwlError::NetworkError(
                format!("Failed to read response from '{}'", url),
                e.to_string(),
            )
        })?;

    let mut cursor = Cursor::new(resp);

    let mut out_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out)
        .map_err(|e| {
            OwlError::FileError(
                format!("Failed to truncate '{}' for writing", out.to_string_lossy()),
                e.to_string(),
            )
        })?;

    copy(&mut cursor, &mut out_file).map_err(|e| {
        OwlError::FileError(
            format!(
                "Failed to copy response from '{}' into '{}'",
                url,
                out.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn ensure_path_from_home(dirs: &[&str], file_str: Option<&str>) -> Result<PathBuf> {
    let mut path = dirs::home_dir().ok_or(OwlError::FileError(
        "Failed to find home dir".into(),
        "None".into(),
    ))?;

    for dir in dirs {
        path.push(dir);
    }

    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| {
            OwlError::FileError(
                format!("Failed to create all dirs in '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })?;
    }

    if let Some(filename) = file_str {
        path.push(filename);
    }

    Ok(path)
}

pub fn extract_archive(archive_file: &Path, out_dir: &Path) -> Result<()> {
    let zip_file = OpenOptions::new()
        .read(true)
        .open(archive_file)
        .map_err(|e| {
            OwlError::FileError(
                format!(
                    "Failed to open zip archive '{}' for reading",
                    archive_file.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;

    let mut zip_archive = ZipArchive::new(zip_file).map_err(|e| {
        OwlError::FileError(
            format!(
                "Failed to parse zip archive '{}'",
                archive_file.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    if !out_dir.exists() {
        fs::create_dir_all(out_dir).map_err(|e| {
            OwlError::FileError(
                format!(
                    "Failed to create all dirs in '{}'",
                    out_dir.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;
    }

    zip_archive.extract(out_dir).map_err(|e| {
        OwlError::FileError(
            format!(
                "Failed to extract zip archive '{}' into '{}'",
                archive_file.to_string_lossy(),
                out_dir.to_string_lossy()
            ),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn find_by_ext(root_dir: &Path, target_ext: &str) -> Result<Vec<PathBuf>> {
    dir_tree(root_dir).map_or_else(Err, |files| {
        let n = files.len();

        let matches = files
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
            .collect::<Vec<PathBuf>>();

        if matches.is_empty() {
            Err(OwlError::FileError(
                format!(
                    "No matches found in '{}' with ext matching '{}'",
                    root_dir.to_string_lossy(),
                    target_ext
                ),
                format!("'{}' files checked", n),
            ))
        } else {
            Ok(matches)
        }
    })
}

pub fn find_by_stem_and_ext(
    root_dir: &Path,
    target_stem: &str,
    target_ext: &str,
) -> Result<PathBuf> {
    dir_tree(root_dir).map_or_else(Err, |files| {
        let n = files.len();

        let file_match = files.into_iter().find(|file| {
            if let Some(stem) = file.file_stem()
                && stem == target_stem
                && let Some(ext) = file.extension()
                && ext == target_ext
            {
                true
            } else {
                false
            }
        });

        match file_match {
            Some(target_file) => Ok(target_file),
            None => Err(OwlError::FileError(
                format!(
                    "No matches found in '{}' matching '{}.{}'",
                    root_dir.to_string_lossy(),
                    target_stem,
                    target_ext
                ),
                format!("'{}' files checked", n),
            )),
        }
    })
}

pub fn read_contents(path: &Path) -> Result<String> {
    if !path.exists() {
        Err(OwlError::FileError(
            format!("Failed to access dir '{}'", path.to_string_lossy()),
            "no such directory <os error 2>".into(),
        ))
    } else if path.is_dir() {
        fs::read_dir(path)
            .map(|dir_read| {
                dir_read
                    .into_iter()
                    .map(|try_entry| match try_entry {
                        Ok(dir_entry) => {
                            let dir_name = dir_entry
                                .file_name()
                                .into_string()
                                .unwrap_or_else(|e| e.to_string_lossy().to_string());

                            if let Ok(ft) = dir_entry.file_type()
                                && ft.is_dir()
                            {
                                format!("{}/", dir_name)
                            } else {
                                dir_name
                            }
                        }
                        Err(_) => "<could not read entry name>".into(),
                    })
                    .collect::<Vec<String>>()
                    .join("\n")
            })
            .map_err(|e| {
                OwlError::FileError(
                    format!("Failed to read entries in dir '{}'", path.to_string_lossy()),
                    e.to_string(),
                )
            })
    } else {
        fs::read_to_string(path).map_err(|e| {
            OwlError::FileError(
                format!("Failed to read from '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })
    }
}

pub fn remove_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| {
            OwlError::FileError(
                format!(
                    "Failed to remove-recursively dir '{}'",
                    path.to_string_lossy()
                ),
                e.to_string(),
            )
        })?;
    } else if path.is_file() {
        fs::remove_file(path).map_err(|e| {
            OwlError::FileError(
                format!("Failed to remove file '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })?;
    }

    Ok(())
}
