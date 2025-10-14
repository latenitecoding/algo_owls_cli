use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write, copy};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, value};
use zip::ZipArchive;

use super::owl_error::{OwlError, file_error, net_error, no_entry_found};

pub fn create_toml_with_entry(
    path: &Path,
    toml_template: &str,
    table: &str,
    name: &str,
    item: &str,
) -> Result<(), OwlError> {
    let mut doc = toml_template
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    doc[table][name] = value(item);

    let toml_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)
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

pub fn ensure_dir_from_home(dir: &str) -> Result<PathBuf, OwlError> {
    let mut dir_path = dirs::home_dir().ok_or(file_error!("$HOME"))?;
    dir_path.push(dir);

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

pub fn get_toml_entry(path: &Path, tables: &[&str], name: &str) -> Result<String, OwlError> {
    let toml_str = fs::read_to_string(&path).map_err(|e| file_error!(e))?;
    let doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    for &table in tables {
        if let Some(entry) = doc[table].get(name) {
            return entry
                .as_value()
                .ok_or(no_entry_found!(name))?
                .as_str()
                .ok_or(no_entry_found!(name))
                .map(|ok| ok.to_string());
        }
    }

    Err(no_entry_found!(name))
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

pub fn update_toml_entry(path: &Path, table: &str, name: &str, item: &str) -> Result<(), OwlError> {
    let toml_str = fs::read_to_string(&path).map_err(|e| file_error!(e))?;
    let mut doc = toml_str
        .parse::<DocumentMut>()
        .map_err(|e| file_error!(e))?;

    if doc[table].get(name).is_none() && table == "personal" {
        // the entry is not in the manifest so it can be appended
        // skips rewriting the whole file
        let manifest_file = OpenOptions::new()
            .append(true)
            .open(&path)
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
        .open(&path)
        .map_err(|e| file_error!(e))?;

    let mut writer = BufWriter::new(manifest_file);

    doc[table][name] = value(item);

    writer
        .write_all(doc.to_string().as_bytes())
        .map_err(|e| file_error!(e))?;
    writer.flush().map_err(|e| file_error!(e))?;

    Ok(())
}
