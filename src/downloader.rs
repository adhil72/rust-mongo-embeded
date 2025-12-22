use anyhow::{anyhow, Result};
use std::env;

#[derive(Debug, Clone)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

#[derive(Debug, Clone)]
pub enum Arch {
    X86_64,
    Aarch64,
}

pub struct MongoUrl {
    pub url: String,
    pub filename: String,
}

pub fn get_os() -> Result<Os> {
    match env::consts::OS {
        "linux" => Ok(Os::Linux),
        "macos" => Ok(Os::MacOs),
        "windows" => Ok(Os::Windows),
        os => Err(anyhow!("Unsupported OS: {}", os)),
    }
}

pub fn get_arch() -> Result<Arch> {
    match env::consts::ARCH {
        "x86_64" => Ok(Arch::X86_64),
        "aarch64" => Ok(Arch::Aarch64),
        arch => Err(anyhow!("Unsupported Architecture: {}", arch)),
    }
}

pub fn get_download_url(version: &str) -> Result<MongoUrl> {
    let os = get_os()?;
    let arch = get_arch()?;

    let (_platform, package_format) = match (&os, &arch) {
        (Os::Linux, Arch::X86_64) => ("linux-x86_64", "tgz"),
        (Os::Linux, Arch::Aarch64) => ("linux-aarch64", "tgz"),
        (Os::MacOs, Arch::X86_64) => ("macos-x86_64", "tgz"),
        (Os::MacOs, Arch::Aarch64) => ("macos-aarch64", "tgz"),
        (Os::Windows, Arch::X86_64) => ("windows-x86_64", "zip"),
        _ => return Err(anyhow!("Unsupported OS/Arch combination")),
    };

    // Note: This is a simplified URL constructor. 
    // Real MongoDB URLs are more complex and depend on specific distros for Linux.
    // We might need to handle specific linux distros. 
    // For now, let's try to find a generic linux binary or handle specific common ones.
    
    // Example: https://fastdl.mongodb.org/linux/mongodb-linux-x86_64-ubuntu2004-7.0.2.tgz
    // Example: https://fastdl.mongodb.org/osx/mongodb-macos-x86_64-7.0.2.tgz
    // Example: https://fastdl.mongodb.org/windows/mongodb-windows-x86_64-7.0.2.zip

    // For generic linux, we often need to specify a distro.
    // Let's assume ubuntu2204 for now for linux x64 as a safeish default or try to detect.
    // Or use the "generic" linux legacy if available, but modern mongo usually target distros.
    
    let base_url = "https://fastdl.mongodb.org";
    
    let _filename = match os {
        Os::Linux => format!("mongodb-linux-{}-{}.{}", "x86_64-ubuntu2204", version, package_format), // HARDCODING ubuntu2204 for x64 linux for now
        Os::MacOs => format!("mongodb-macos-{}-{}.{}", "x86_64", version, package_format), // Need to fix arch for mac
        Os::Windows => format!("mongodb-windows-x86_64-{}.{}", version, package_format),
    };

    // Refined logic
    let url = match (&os, &arch) {
        (Os::Linux, Arch::X86_64) => format!("{}/linux/mongodb-linux-x86_64-ubuntu2204-{}.tgz", base_url, version),
        (Os::Linux, Arch::Aarch64) => format!("{}/linux/mongodb-linux-aarch64-ubuntu2204-{}.tgz", base_url, version),
        (Os::MacOs, Arch::X86_64) => format!("{}/osx/mongodb-macos-x86_64-{}.tgz", base_url, version),
        (Os::MacOs, Arch::Aarch64) => format!("{}/osx/mongodb-macos-aarch64-{}.tgz", base_url, version),
        (Os::Windows, Arch::X86_64) => format!("{}/windows/mongodb-windows-x86_64-{}.zip", base_url, version),
        _ => return Err(anyhow!("Unsupported OS/Arch combination")),
    };

    let filename = url.split('/').last().unwrap().to_string();

    Ok(MongoUrl {
        url,
        filename,
    })
}


pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: Option<f32>,
}

pub async fn download_file(url: &str, destination: &std::path::Path) -> Result<()> {
    download_file_with_callback(url, destination, |_| {}).await
}

pub async fn download_file_with_callback<F>(
    url: &str,
    destination: &std::path::Path,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(DownloadProgress),
{
    use std::io::Write;
    use std::fs::File;

    let response = reqwest::get(url).await?;
    let total = response.content_length();

    let mut part_path = destination.to_path_buf();
    part_path.set_extension("part");

    let mut file = File::create(&part_path)?;
    let mut downloaded: u64 = 0;

    let mut stream = response;
    while let Some(chunk) = stream.chunk().await? {
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        let percentage = total.map(|t| (downloaded as f32 / t as f32) * 100.0);
        callback(DownloadProgress {
            downloaded,
            total,
            percentage,
        });
    }

    std::fs::rename(part_path, destination)?;

    Ok(())
}
