use super::{Uri, fs_utils};
use crate::common::{OwlError, Result};
use reqwest;
use std::cmp::Ordering;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use toml_edit::{DocumentMut, Table, value};
use url::Url;

pub async fn check_updates(
    remote_manifest_url: &Url,
    manifest_path: &Path,
) -> Result<(Ordering, Ordering)> {
    let (local_version, local_timestamp) = get_manifest_version_timestamp(manifest_path)?;

    let remote_doc = request_toml(remote_manifest_url).await?;

    let remote_version = remote_doc["manifest"]["version"]
        .as_str()
        .map(String::from)
        .ok_or(OwlError::TomlError(
            format!(
                "Failed to extract manifest version from remote '{}'",
                remote_manifest_url
            ),
            "None".into(),
        ))?;
    let remote_timestamp = remote_doc["manifest"]["timestamp"]
        .as_str()
        .map(String::from)
        .ok_or(OwlError::TomlError(
            format!(
                "Failed to extract manifest timestamp from remote '{}'",
                remote_manifest_url
            ),
            "None".into(),
        ))?;

    Ok((
        compare_stamps(&local_version, &remote_version)?,
        compare_stamps(&local_timestamp, &remote_timestamp)?,
    ))
}

pub async fn commit_doc(
    manifest_path: &Path,
    prompt_dir: &Path,
    ext_name: &str,
    remote_doc: &DocumentMut,
    local_doc: &mut DocumentMut,
    and_fetch_to_tmp: Option<&Path>,
) -> Result<()> {
    if let Some(personal_table) = remote_doc["personal"].as_table() {
        let mut quest_path = manifest_path
            .parent()
            .expect("manifest file to have parent owlgo directory")
            .to_path_buf();

        for (quest_name, quest_uri) in personal_table.iter() {
            local_doc["personal"][quest_name] = quest_uri.clone();

            if let Some(tmp_archive) = and_fetch_to_tmp {
                quest_path.push(quest_name);

                let quest_uri_str = quest_uri.as_str().ok_or(OwlError::TomlError(
                    format!(
                        "Invalid entry for '{}' in table 'personal' in extension '{}'",
                        quest_name, ext_name
                    ),
                    "None".into(),
                ))?;

                match Uri::try_from(quest_uri_str)? {
                    Uri::Local(path) => {
                        fs_utils::extract_archive(&path, tmp_archive)?;
                        fs_utils::remove_path(tmp_archive)?
                    }
                    Uri::Remote(url) => {
                        fs_utils::download_archive(&url, tmp_archive, &quest_path).await?
                    }
                };

                quest_path.pop();
            }
        }
    }

    if let Some(prompt_table) = remote_doc["prompts"].as_table() {
        let mut prompt_path = prompt_dir.to_path_buf();

        for (prompt_name, prompt_uri) in prompt_table.iter() {
            local_doc["prompts"][prompt_name] = prompt_uri.clone();

            if and_fetch_to_tmp.is_some() {
                let prompt_uri_str = prompt_uri.as_str().ok_or(OwlError::TomlError(
                    format!(
                        "Invalid entry for '{}' in table 'prompts' in extension '{}'",
                        prompt_name, ext_name
                    ),
                    "None".into(),
                ))?;

                prompt_path.push(prompt_name);

                match Uri::try_from(prompt_uri_str)? {
                    Uri::Local(path) => fs_utils::copy_file(&path, &prompt_path)?,
                    Uri::Remote(url) => fs_utils::download_file(&url, &prompt_path).await?,
                };

                prompt_path.pop();
            }
        }
    }

    Ok(())
}

pub async fn commit_extension(
    manifest_path: &Path,
    prompt_dir: &Path,
    ext_name: &str,
    ext_uri: &Uri,
    ext_doc: &DocumentMut,
    manifest_doc: &mut DocumentMut,
    and_fetch_to_tmp: Option<&Path>,
) -> Result<()> {
    manifest_doc["extensions"][ext_name] = ext_doc["manifest"]["timestamp"].clone();

    let ext_uri_key = format!("{}.uri", ext_name);
    match ext_uri {
        Uri::Local(ext_path) => {
            manifest_doc["ext_uri"][ext_uri_key] = value(ext_path.to_str().ok_or(
                OwlError::FileError("Invalid extension path".into(), "None".into()),
            )?)
        }
        Uri::Remote(ext_url) => manifest_doc["ext_uri"][ext_uri_key] = value(ext_url.as_str()),
    }

    commit_doc(
        manifest_path,
        prompt_dir,
        ext_name,
        ext_doc,
        manifest_doc,
        and_fetch_to_tmp,
    )
    .await?;

    write_manifest(manifest_doc, manifest_path)
}

pub fn compare_stamps(s1: &str, s2: &str) -> Result<Ordering> {
    for (s, t) in s1.split('.').zip(s2.split('.')) {
        let s_num = s.parse::<usize>().map_err(|e| {
            OwlError::TomlError(
                format!("Failed to parse timestamp '{}' as (usize,,)", s1),
                e.to_string(),
            )
        })?;
        let t_num = t.parse::<usize>().map_err(|e| {
            OwlError::TomlError(
                format!("Failed to parse timestamp '{}' as (usize,,)", s2),
                e.to_string(),
            )
        })?;

        if s_num < t_num {
            return Ok(Ordering::Less);
        }
    }

    if s1 == s2 {
        Ok(Ordering::Equal)
    } else {
        Ok(Ordering::Greater)
    }
}

pub fn create_toml(path: &Path, toml_template: &str) -> Result<()> {
    let toml_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|e| {
            OwlError::FileError(
                format!("Faile to truncate '{}' for writing", path.to_string_lossy()),
                e.to_string(),
            )
        })?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(toml_template.trim().as_bytes())
        .map_err(|e| {
            OwlError::FileError(
                format!("Failed to write TOML doc to '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })?;
    writer.flush().map_err(|e| {
        OwlError::FileError(
            format!("Failed to flush bytes to '{}'", path.to_string_lossy()),
            e.to_string(),
        )
    })?;

    Ok(())
}

pub fn get_embedded_version(toml_str: &str) -> Result<String> {
    let doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| OwlError::TomlError("Failed to parse str as TOML".into(), e.to_string()))?;

    doc["manifest"]["version"]
        .as_str()
        .map(String::from)
        .ok_or(OwlError::TomlError(
            "Failed to extract embedded TOML version".into(),
            "None".into(),
        ))
}

pub fn get_manifest_ai_sdk(manifest_path: &Path) -> Result<(String, String)> {
    let doc = get_manifest_header_doc(manifest_path)?;

    let ai_sdk =
        doc["manifest"]["ai_sdk"]
            .as_str()
            .map(String::from)
            .ok_or(OwlError::TomlError(
                "Failed not extract entry 'ai_sdk' in table 'manifest'".into(),
                "None".into(),
            ))?;
    let api_key = doc["manifest"]["api_key"]
        .as_str()
        .map(String::from)
        .ok_or(OwlError::TomlError(
            "Failed not extract entry 'api_key' in table 'manifest'".into(),
            "None".into(),
        ))?;

    Ok((ai_sdk, api_key))
}

pub fn get_manifest_header_doc(manifest_path: &Path) -> Result<DocumentMut> {
    let file = File::open(manifest_path)
        .map_err(|e| OwlError::FileError("Failed to open manifest".into(), e.to_string()))?;

    let reader = BufReader::new(file);

    let mut toml_str = String::new();
    for line in reader.lines().take(5) {
        match line {
            Ok(line_str) => {
                toml_str.push_str(&line_str);
                toml_str.push('\n');
            }
            Err(e) => {
                return Err(OwlError::TomlError(
                    "Failed to read manifest header".into(),
                    e.to_string(),
                ));
            }
        }
    }

    toml_str.parse::<DocumentMut>().map_err(|e| {
        OwlError::TomlError(
            "Failed to parse manifest header as TOML".into(),
            e.to_string(),
        )
    })
}

pub fn get_manifest_version_timestamp(manifest_path: &Path) -> Result<(String, String)> {
    let doc = get_manifest_header_doc(manifest_path)?;

    let version = doc["manifest"]["version"]
        .as_str()
        .map(String::from)
        .ok_or(OwlError::TomlError(
            "Failed to extract 'version' from table 'manifest'".into(),
            "None".into(),
        ))?;
    let timestamp = doc["manifest"]["timestamp"]
        .as_str()
        .map(String::from)
        .ok_or(OwlError::TomlError(
            "Failed to extract 'timestamp' from table 'manifest'".into(),
            "None".into(),
        ))?;

    Ok((version, timestamp))
}

pub fn read_toml(path: &Path) -> Result<DocumentMut> {
    fs::read_to_string(path)
        .map_err(|e| {
            OwlError::FileError(
                format!("Failed to read '{}'", path.to_string_lossy()),
                e.to_string(),
            )
        })?
        .parse::<DocumentMut>()
        .map_err(|e| {
            OwlError::TomlError(
                format!("Failed to parse '{}' as TOML", path.to_string_lossy()),
                e.to_string(),
            )
        })
}

pub async fn request_toml(url: &Url) -> Result<DocumentMut> {
    reqwest::get(url.as_str())
        .await
        .map_err(|e| {
            OwlError::NetworkError(
                format!("Failed to request '{}'", url.as_str()),
                e.to_string(),
            )
        })?
        .text()
        .await
        .map_err(|e| {
            OwlError::NetworkError(
                format!("Failed to read response from '{}'", url.as_str()),
                e.to_string(),
            )
        })?
        .parse::<DocumentMut>()
        .map_err(|e| {
            OwlError::TomlError(
                format!("Failed to parse response from '{}' as TOML", url.as_str()),
                e.to_string(),
            )
        })
}

pub async fn update_extensions(
    manifest_path: &Path,
    prompt_path: &Path,
    and_fetch_to_tmp: &Path,
) -> Result<()> {
    let mut manifest_doc = read_toml(manifest_path)?;

    if let Some(ext_table) = manifest_doc["extensions"].as_table() {
        let mut tmp_doc = DocumentMut::new();
        tmp_doc["extensions"] = Table::new().into();
        tmp_doc["personal"] = Table::new().into();
        tmp_doc["prompts"] = Table::new().into();

        for (ext_name, ext_timestamp) in ext_table.iter() {
            let ext_uri_key = format!("{}.uri", ext_name);

            let ext_uri_str =
                ext_table["ext_uri"][&ext_uri_key]
                    .as_str()
                    .ok_or(OwlError::TomlError(
                        format!(
                            "Invalid entry for '{}' in table 'ext_uri' in extension '{}'",
                            ext_uri_key, ext_name
                        ),
                        "None".into(),
                    ))?;

            let remote_doc = match Uri::try_from(ext_uri_str)? {
                Uri::Local(path) => read_toml(&path)?,
                Uri::Remote(url) => request_toml(&url).await?,
            };

            let remote_ext_timestamp =
                remote_doc["manifest"]["timestamp"]
                    .as_str()
                    .ok_or(OwlError::TomlError(
                        format!(
                            "Invalid entry for 'timestamp' in table 'manifest' from remote extension '{}'",
                            ext_name
                        ),
                        "None".into(),
                    ))?;

            let ext_timestamp_str = ext_timestamp.as_str().ok_or(OwlError::TomlError(
                format!(
                    "Invalid entry for 'timestamp' in table 'manifest' in local extension '{}'",
                    ext_name
                ),
                "None".into(),
            ))?;

            if compare_stamps(ext_timestamp_str, remote_ext_timestamp)? == Ordering::Less {
                tmp_doc["extensions"][ext_name] = value(remote_ext_timestamp);

                commit_doc(
                    manifest_path,
                    prompt_path,
                    ext_name,
                    &remote_doc,
                    &mut tmp_doc,
                    Some(and_fetch_to_tmp),
                )
                .await?;
            }
        }

        if let Some(tmp_ext_table) = tmp_doc["extensions"].as_table() {
            for (key, item) in tmp_ext_table.iter() {
                manifest_doc["extensions"][key] = item.clone();
            }
        }

        if let Some(tmp_personal_table) = tmp_doc["personal"].as_table() {
            for (key, item) in tmp_personal_table.iter() {
                manifest_doc["personal"][key] = item.clone();
            }
        }

        if let Some(tmp_prompt_table) = tmp_doc["prompts"].as_table() {
            for (key, item) in tmp_prompt_table.iter() {
                manifest_doc["prompts"][key] = item.clone();
            }
        }
    }

    write_manifest(&manifest_doc, manifest_path)
}

pub async fn update_manifest(
    header_url: &Url,
    manifest_url: &Url,
    manifest_path: &Path,
    prompt_dir: &Path,
    tmp_archive: &Path,
) -> Result<()> {
    if !manifest_path.exists() {
        println!("no manifest...");
        println!("downloading manifest...");

        let remote_doc = request_toml(manifest_url).await?;
        write_manifest(&remote_doc, manifest_path)?;

        println!("updating extensions...");

        update_extensions(manifest_path, prompt_dir, tmp_archive).await?
    }

    let (version_order, timestamp_order) = check_updates(header_url, manifest_path).await?;

    if timestamp_order == Ordering::Less {
        println!("manifest out of date...");
        println!("updating manifest...");

        let mut manifest_doc = read_toml(manifest_path)?;
        let remote_doc = request_toml(manifest_url).await?;

        manifest_doc["manifest"] = remote_doc["manifest"].clone();
        manifest_doc["quests"] = remote_doc["quests"].clone();

        write_manifest(&manifest_doc, manifest_path)?;
    }

    println!("updating extensions...");

    update_extensions(manifest_path, prompt_dir, tmp_archive).await?;

    if version_order == Ordering::Less {
        println!("owlgo out of date...");
        println!("run `cargo install --force owlgo`")
    }

    Ok(())
}

pub fn write_manifest(manifest_doc: &DocumentMut, manifest_path: &Path) -> Result<()> {
    let manifest_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(manifest_path)
        .map_err(|e| {
            OwlError::FileError(
                "Failed to truncate manifest for writing".into(),
                e.to_string(),
            )
        })?;

    let mut writer = BufWriter::new(manifest_file);

    writer
        .write_all(manifest_doc.to_string().trim().as_bytes())
        .map_err(|e| {
            OwlError::FileError("Failed to write TOML doc to manifest".into(), e.to_string())
        })?;
    writer.flush().map_err(|e| {
        OwlError::FileError("Failed to flush bytes to manifest".into(), e.to_string())
    })?;

    Ok(())
}
