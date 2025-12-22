use anyhow::{anyhow, Result};
use std::path::Path;
use std::fs::File;
use flate2::read::GzDecoder;
use tar::Archive;

pub fn extract(archive_path: &Path, extract_to: &Path) -> Result<()> {
    if !extract_to.exists() {
        std::fs::create_dir_all(extract_to)?;
    }

    let extension = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| anyhow!("Unknown file extension"))?;

    match extension {
        "tgz" | "gz" => {
            let file = File::open(archive_path)?;
            let tar = GzDecoder::new(file);
            let mut archive = Archive::new(tar);
            archive.unpack(extract_to)?;
        }
        "zip" => {
            let file = File::open(archive_path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            archive.extract(extract_to)?;
        }
        _ => return Err(anyhow!("Unsupported archive format: {}", extension)),
    }

    Ok(())
}
