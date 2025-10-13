use std::fs;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use zip::ZipArchive;

pub fn download_file(url: &str, out: &str) -> Result<(), String> {
    let mut resp = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
    let mut file = File::create(out).map_err(|e| e.to_string())?;
    copy(&mut resp, &mut file).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn extract_archive(filename: &str, dir: &str) -> Result<(), String> {
    let zip_file = File::open(filename).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(zip_file).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    archive.extract(dir).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn remove_path(file_or_dir: &str) -> Result<(), String> {
    let path = Path::new(file_or_dir);
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;

    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    } else if metadata.is_file() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }

    Ok(())
}
