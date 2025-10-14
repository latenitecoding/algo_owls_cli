use std::fs;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use zip::ZipArchive;

use super::owl_error::{OwlError, file_error, net_error};

pub fn download_file(url: &str, out: &str) -> Result<(), OwlError> {
    let mut resp = reqwest::blocking::get(url).map_err(|e| net_error!(e))?;
    let mut file = File::create(out).map_err(|e| file_error!(e))?;
    copy(&mut resp, &mut file).map_err(|e| file_error!(e))?;

    Ok(())
}

pub fn extract_archive(filename: &str, dir: &str) -> Result<(), OwlError> {
    let zip_file = File::open(filename).map_err(|e| file_error!(e))?;
    let mut archive = ZipArchive::new(zip_file).map_err(|e| file_error!(e))?;
    fs::create_dir_all(dir).map_err(|e| file_error!(e))?;
    archive.extract(dir).map_err(|e| file_error!(e))?;

    Ok(())
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
