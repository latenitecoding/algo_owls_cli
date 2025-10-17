use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write, copy};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Table, value};
use zip::ZipArchive;

use super::owl_error::{
    OwlError, check_item, check_manifest, check_path, file_error, file_not_found, net_error,
    no_entry_found,
};

pub fn as_ans_file(in_file: &str) -> Result<String, OwlError> {
    let in_path = Path::new(in_file);

    let target_stem = in_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or(file_error!(format!("no parent of: '{}'", in_file)))?;

    let ans_file = format!("{}.ans", target_stem);

    let mut ans_path = in_path
        .parent()
        .ok_or(file_error!(format!("no parent of: '{}'", in_file)))?
        .to_path_buf();

    ans_path.push(ans_file);

    if !ans_path.exists() {
        Err(file_not_found!(check_path!(ans_path)?))
    } else {
        Ok(ans_path
            .to_str()
            .ok_or(file_error!(check_path!(ans_path)?))?
            .to_string())
    }
}

pub fn cat_file(filepath: &str) -> Result<String, OwlError> {
    let path = Path::new(filepath);

    if !path.exists() {
        return Err(file_not_found!(filepath));
    }

    let mut file = File::open(path).map_err(|e| file_error!(e))?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .map_err(|e| file_error!(e))?;

    Ok(buffer)
}

pub fn check_for_updates(
    url: &str,
    local_version: &str,
    local_timestamp: &str,
) -> Result<(bool, bool), OwlError> {
    let doc = load_toml_doc(url, false)?;

    let remote_version = check_manifest!(doc["manifest"], "version")?;
    let remote_timestamp = check_manifest!(doc["manifest"], "timestamp")?;

    Ok((
        compare_stamps(local_version, &remote_version)?,
        compare_stamps(local_timestamp, &remote_timestamp)?,
    ))
}

pub fn commit_manifest(
    manifest_path: &str,
    name: &str,
    uri: &str,
    and_fetch: Option<&str>,
    is_local: bool,
) -> Result<(), OwlError> {
    let mut local_doc = load_toml_doc(manifest_path, true)?;
    let remote_doc = load_toml_doc(uri, is_local)?;

    let ext_uri = if is_local {
        format!("{}.path", name)
    } else {
        format!("{}.url", name)
    };

    local_doc["extensions"][name] = remote_doc["manifest"]["timestamp"].clone();
    local_doc["ext_uri"][ext_uri] = value(uri);

    if let Some(personal_table) = remote_doc["personal"].as_table() {
        let mut quest_path = Path::new(manifest_path)
            .parent()
            .ok_or(file_error!(format!("no parent of {}", manifest_path)))?
            .to_path_buf();

        for (key, item) in personal_table.iter() {
            local_doc["personal"][key] = item.clone();

            if let Some(tmp_archive) = and_fetch {
                quest_path.push(key);

                let url = check_item!(item, key)?;
                download_archive(&url, tmp_archive, check_path!(quest_path)?)?;

                quest_path.pop();
            }
        }
    }

    let toml_file = OpenOptions::new()
        .write(true)
        .open(manifest_path)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(local_doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn compare_stamps(s1: &str, s2: &str) -> Result<bool, OwlError> {
    for (s, t) in s1.split('.').zip(s2.split('.')) {
        let s_num = s.parse::<usize>().map_err(|e| file_error!(e))?;
        let t_num = t.parse::<usize>().map_err(|e| file_error!(e))?;

        if s_num < t_num {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn copy_file(src: &str, dst: &str) -> Result<(), OwlError> {
    if !Path::new(src).exists() {
        return Err(file_not_found!(src));
    }

    let mut src_file = OpenOptions::new()
        .read(true)
        .open(src)
        .map_err(|e| file_error!(e))?;

    if Path::new(dst).exists() {
        remove_path(dst)?;
    }

    let mut dst_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dst)
        .map_err(|e| file_error!(e))?;

    copy(&mut src_file, &mut dst_file).map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn create_toml(filepath: &str, toml_template: &str) -> Result<(), OwlError> {
    let toml_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(filepath)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(toml_template.trim().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn create_toml_with_entry(
    filepath: &str,
    toml_template: &str,
    table: &str,
    name: &str,
    item: &str,
) -> Result<(), OwlError> {
    let mut doc = toml_template
        .trim()
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    doc[table][name] = value(item);

    let toml_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(filepath)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn download_archive(url: &str, tmp_archive: &str, out_dir: &str) -> Result<(), OwlError> {
    download_file(url, tmp_archive)?;
    extract_archive(tmp_archive, out_dir)?;
    remove_path(tmp_archive)
}

pub fn download_file(url: &str, out: &str) -> Result<(), OwlError> {
    let mut resp = reqwest::blocking::get(url).map_err(|e| net_error!(e))?;
    let mut file = File::create(out).map_err(|e| file_error!(e))?;
    copy(&mut resp, &mut file).map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn ensure_dir_from_home(dirs: &[&str]) -> Result<PathBuf, OwlError> {
    let mut dir_path = dirs::home_dir().ok_or(file_error!("$HOME"))?;
    for dir in dirs {
        dir_path.push(dir);
    }

    if !dir_path.exists() {
        fs::create_dir_all(&dir_path).map_err(|e| file_error!(e))?;
    }

    Ok(dir_path)
}

pub fn extract_archive(filename: &str, dir: &str) -> Result<(), OwlError> {
    let zip_file = File::open(filename).map_err(|e| file_error!(e))?;
    let mut archive = ZipArchive::new(zip_file).map_err(|e| file_error!(e))?;
    fs::create_dir_all(dir).map_err(|e| file_error!(e))?;
    archive.extract(dir).map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn extract_toml_version(toml_template: &str) -> Result<String, OwlError> {
    let doc = toml_template
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    check_manifest!(doc["manifest"], "version")
}

pub fn find_by_ext(root_dir: String, target_ext: &str) -> Result<Vec<String>, OwlError> {
    let mut test_cases: Vec<String> = Vec::new();

    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root_dir);

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(dir).map_err(|e| file_error!(e))? {
            let path = entry.map_err(|e| file_error!(e))?.path();

            if path.is_dir() {
                queue.push_back(check_path!(path)?.to_string());
            } else if path.is_file()
                && let Some(ext) = path.extension().and_then(OsStr::to_str)
                && ext == target_ext
            {
                test_cases.push(check_path!(path)?.to_string());
            }
        }
    }

    Ok(test_cases)
}

pub fn find_by_stem_and_ext(
    root_dir: String,
    target_stem: &str,
    target_ext: &str,
) -> Result<String, OwlError> {
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root_dir);

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(dir).map_err(|e| file_error!(e))? {
            let path = entry.map_err(|e| file_error!(e))?.path();

            if path.is_dir() {
                queue.push_back(check_path!(path)?.to_string());
            } else if path.is_file()
                && let Some(stem) = path.file_stem().and_then(OsStr::to_str)
                && stem == target_stem
                && let Some(ext) = path.extension().and_then(OsStr::to_str)
                && ext == target_ext
            {
                return Ok(path.to_str().ok_or(file_error!(target_stem))?.to_string());
            }
        }
    }

    Err(file_not_found!(target_stem))
}

pub fn get_toml_entry(filepath: &str, tables: &[&str], name: &str) -> Result<String, OwlError> {
    let doc = load_toml_doc(filepath, true)?;

    for &table in tables {
        if doc[table].get(name).is_some() {
            return check_manifest!(doc[table], name);
        }
    }

    Err(no_entry_found!(name))
}

pub fn get_toml_version_timestamp(filepath: &str) -> Result<(String, String), OwlError> {
    let path = Path::new(filepath);
    let file = File::open(path).map_err(|e| file_error!(e))?;

    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut toml_str = String::new();
    for _ in 0..3 {
        if let Some(Ok(line)) = lines.next() {
            toml_str.push_str(&line);
            toml_str.push('\n');
        }
    }

    let doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    let version = check_manifest!(doc["manifest"], "version")?;
    let timestamp = check_manifest!(doc["manifest"], "timestamp")?;

    Ok((version, timestamp))
}

pub fn list_dir(root_dir: String) -> Result<String, OwlError> {
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root_dir);

    let mut buffer = String::new();

    while let Some(dir) = queue.pop_front() {
        for entry in fs::read_dir(dir).map_err(|e| file_error!(e))? {
            let path = entry.map_err(|e| file_error!(e))?.path();

            if path.is_dir() {
                queue.push_back(check_path!(path)?.to_string());
            } else if path.is_file() {
                buffer.push_str(check_path!(path)?);
                buffer.push('\n');
            }
        }
    }

    Ok(buffer)
}

fn load_toml_doc(uri: &str, is_local: bool) -> Result<DocumentMut, OwlError> {
    let toml_str = if is_local {
        fs::read_to_string(uri).map_err(|e| file_error!(e))?
    } else {
        let mut resp = reqwest::blocking::get(uri).map_err(|e| net_error!(e))?;

        let mut remote_toml_str = String::new();
        resp.read_to_string(&mut remote_toml_str)
            .map_err(|e| file_error!(e))?;

        remote_toml_str
    };

    toml_str.parse::<DocumentMut>().map_err(|e| file_error!(e))
}

pub fn remove_path(file_or_dir: &str) -> Result<(), OwlError> {
    let path = Path::new(file_or_dir);
    let metadata = fs::metadata(path).map_err(|e| file_error!(e))?;

    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|e| file_error!(e))?;
    } else if metadata.is_file() {
        fs::remove_file(path).map_err(|e| file_error!(e))?;
    }

    Ok(())
}

pub fn update_extensions(manifest_path: &str, tmp_archive: &str) -> Result<(), OwlError> {
    let mut local_doc = load_toml_doc(manifest_path, true)?;

    if let Some(ext_table) = local_doc["extensions"].as_table() {
        let mut quest_path = Path::new(manifest_path)
            .parent()
            .ok_or(file_error!(format!("no parent of {}", manifest_path)))?
            .to_path_buf();

        let mut tmp_doc = DocumentMut::new();
        tmp_doc["extensions"] = Table::new().into();
        tmp_doc["ext_uri"] = Table::new().into();
        tmp_doc["personal"] = Table::new().into();

        for (ext_name, timestamp) in ext_table.iter() {
            let ext_path = format!("{}.path", ext_name);
            let ext_url = format!("{}.url", ext_name);

            let is_local = local_doc["ext_uri"].get(&ext_path).is_some();
            let is_remote = local_doc["ext_uri"].get(&ext_url).is_some();

            if !is_local && !is_remote {
                return Err(file_not_found!(ext_name));
            }

            let uri = if is_local {
                check_manifest!(local_doc["ext_uri"], &ext_path)?
            } else {
                check_manifest!(local_doc["ext_uri"], &ext_url)?
            };

            let remote_doc = load_toml_doc(&uri, is_local)?;

            if !compare_stamps(
                &check_item!(timestamp, "timestamp")?,
                &check_manifest!(remote_doc["manifest"], "timestamp")?,
            )? {
                continue;
            }

            tmp_doc["extensions"][ext_name] =
                value(check_manifest!(remote_doc["manifest"], "timestamp")?);
            if is_local {
                tmp_doc["ext_uri"][ext_path] = value(uri);
            } else {
                tmp_doc["ext_uri"][ext_url] = value(uri);
            }

            if let Some(personal_table) = remote_doc["personal"].as_table() {
                for (key, item) in personal_table.iter() {
                    tmp_doc["personal"][key] = item.clone();

                    quest_path.push(key);

                    let url = check_item!(item, key)?;
                    if quest_path.exists() {
                        remove_path(check_path!(quest_path)?)?;
                    }
                    download_archive(&url, tmp_archive, check_path!(quest_path)?)?;

                    quest_path.pop();
                }
            }
        }

        if let Some(tmp_ext_table) = tmp_doc["extensions"].as_table() {
            for (key, item) in tmp_ext_table.iter() {
                local_doc["extensions"][key] = item.clone();
            }
        }

        if let Some(tmp_uri_table) = tmp_doc["ext_uri"].as_table() {
            for (key, item) in tmp_uri_table.iter() {
                local_doc["ext_uri"][key] = item.clone();
            }
        }

        if let Some(tmp_personal_table) = tmp_doc["personal"].as_table() {
            for (key, item) in tmp_personal_table.iter() {
                local_doc["personal"][key] = item.clone();
            }
        }
    }

    let toml_file = OpenOptions::new()
        .write(true)
        .open(manifest_path)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(local_doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn update_toml(filepath: &str, url: &str) -> Result<(), OwlError> {
    let mut local_doc = load_toml_doc(filepath, true)?;
    let remote_doc = load_toml_doc(url, false)?;

    local_doc["manifest"] = remote_doc["manifest"].clone();
    local_doc["quests"] = remote_doc["quests"].clone();

    let manifest_file = OpenOptions::new()
        .write(true)
        .open(filepath)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(manifest_file);

    writer
        .write_all(local_doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn update_toml_entry(
    filepath: &str,
    table: &str,
    name: &str,
    item: &str,
) -> Result<(), OwlError> {
    let toml_str = fs::read_to_string(filepath).map_err(|e| file_error!(e))?;
    let mut doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    if doc[table].get(name).is_none() && table == "personal" {
        // the entry is not in the manifest so it can be appended
        // skips rewriting the whole file
        let manifest_file = OpenOptions::new()
            .append(true)
            .open(filepath)
            .map_err(|e| file_error!(e))?;

        let mut writer = BufWriter::new(manifest_file);

        let entry = format!("{} = \"{}\"\n", name, item);

        writer
            .write_all(entry.as_bytes())
            .map_err(|e| file_error!(e))?;
        writer.flush().map_err(|e| file_error!(e))?;

        return Ok(());
    }

    let manifest_file = OpenOptions::new()
        .write(true)
        .open(filepath)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(manifest_file);

    doc[table][name] = value(item);

    writer
        .write_all(doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}
