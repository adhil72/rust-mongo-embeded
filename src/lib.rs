pub mod downloader;
pub mod extractor;
pub mod process;

use anyhow::Result;
use std::path::PathBuf;
use directories::ProjectDirs;

use crate::downloader::{get_download_url, download_file_with_callback, get_os};
use crate::extractor::extract;
use crate::process::MongoProcess;

pub use crate::downloader::DownloadProgress;

pub enum InitStatus {
    CheckingDB,
    ValidatingInstallation,
    Downloading,
    DownloadProgress(DownloadProgress),
    DBInitialized,
}

pub struct MongoEmbedded {
    pub version: String,
    pub download_path: PathBuf,
    pub extract_path: PathBuf,
    pub db_path: PathBuf,
    pub port: u16,
    pub bind_ip: String,
}


impl MongoEmbedded {
    pub fn new(version: &str) -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "mongo", "embedded")
            .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
        
        let cache_dir = proj_dirs.cache_dir();
        let data_dir = proj_dirs.data_dir();

        Ok(Self {
            version: version.to_string(),
            download_path: cache_dir.join("downloads"),
            extract_path: cache_dir.join("extracted"),
            db_path: data_dir.join("db"),
            port: 27017,
            bind_ip: "127.0.0.1".to_string(),
        })
    }

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn set_bind_ip(mut self, bind_ip: &str) -> Self {
        self.bind_ip = bind_ip.to_string();
        self
    }

    pub fn set_db_path(mut self, path: PathBuf) -> Self {
        self.db_path = path;
        self
    }

    pub fn is_installed(&self) -> bool {
        let extract_target = self.extract_path.join(self.version.as_str());
        extract_target.exists()
    }

    pub async fn start(&self) -> Result<MongoProcess> {
        self.start_with_progress(|_| {}).await
    }

    pub async fn start_with_progress<F>(&self, mut callback: F) -> Result<MongoProcess>
    where
        F: FnMut(InitStatus),
    {
        callback(InitStatus::CheckingDB);
        let mongo_url = get_download_url(&self.version)?;
        let download_target = self.download_path.join(&mongo_url.filename);

        callback(InitStatus::ValidatingInstallation);
        if !download_target.exists() {
            if !self.download_path.exists() {
                std::fs::create_dir_all(&self.download_path)?;
            }
            callback(InitStatus::Downloading);
            download_file_with_callback(&mongo_url.url, &download_target, |progress| {
                callback(InitStatus::DownloadProgress(progress));
            }).await?;
        }

        let extract_target = self.extract_path.join(self.version.as_str());
        if !extract_target.exists() {
            extract(&download_target, &extract_target)?;
        }

        let os = get_os()?;
        
        let process = MongoProcess::start(&extract_target, self.port, &self.db_path, &os, &self.bind_ip)?;
        callback(InitStatus::DBInitialized);
        Ok(process)
    }
}

