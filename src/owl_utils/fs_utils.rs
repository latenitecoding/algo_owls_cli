use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write, copy};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, value};
use zip::ZipArchive;

use super::owl_error::{
    OwlError, check_manifest, check_path, file_error, file_not_found, net_error, no_entry_found,
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
    let mut resp = reqwest::blocking::get(url).map_err(|e| net_error!(e))?;

    let mut toml_str = String::new();
    resp.read_to_string(&mut toml_str)
        .map_err(|e| file_error!(e))?;

    let doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    let remote_version = check_manifest!(doc["manifest"], "version")?;
    let remote_timestamp = check_manifest!(doc["manifest"], "timestamp")?;

    Ok((
        compare_stamps(local_version, &remote_version)?,
        compare_stamps(local_timestamp, &remote_timestamp)?,
    ))
}

pub fn compare_stamps(s1: &str, s2: &str) -> Result<bool, OwlError> {
    for (s, t) in s1.split('.').into_iter().zip(s2.split('.').into_iter()) {
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
        .write(true)
        .open(dst)
        .map_err(|e| file_error!(e))?;

    copy(&mut src_file, &mut dst_file).map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn create_toml(filepath: &str, toml_template: &str) -> Result<(), OwlError> {
    let toml_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(filepath)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(toml_template.trim().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    return Ok(());
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
        .write(true)
        .open(filepath)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(toml_file);

    writer
        .write_all(doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    return Ok(());
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
    let toml_str = fs::read_to_string(filepath).map_err(|e| file_error!(e))?;
    let doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

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

pub fn update_toml(filepath: &str, url: &str) -> Result<(), OwlError> {
    let mut resp = reqwest::blocking::get(url).map_err(|e| net_error!(e))?;

    let mut remote_toml_str = String::new();
    resp.read_to_string(&mut remote_toml_str)
        .map_err(|e| file_error!(e))?;

    let remote_doc = remote_toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    let local_toml_str = fs::read_to_string(filepath).map_err(|e| file_error!(e))?;
    let mut local_doc = local_toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

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
