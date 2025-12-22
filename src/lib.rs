pub mod downloader;
pub mod extractor;
pub mod process;

use anyhow::Result;
use std::path::PathBuf;
use directories::ProjectDirs;

use crate::downloader::{get_download_url, download_file, get_os};
use crate::extractor::extract;
use crate::process::MongoProcess;

pub struct MongoEmbedded {
    pub version: String,
    pub download_path: PathBuf,
    pub extract_path: PathBuf,
    pub db_path: PathBuf,
    pub port: u16,
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
        })
    }

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn set_db_path(mut self, path: PathBuf) -> Self {
        self.db_path = path;
        self
    }

    pub async fn start(&self) -> Result<MongoProcess> {
        let mongo_url = get_download_url(&self.version)?;
        let download_target = self.download_path.join(&mongo_url.filename);

        if !download_target.exists() {
            if !self.download_path.exists() {
                std::fs::create_dir_all(&self.download_path)?;
            }
            // println!("Downloading MongoDB from {}", mongo_url.url);
            download_file(&mongo_url.url, &download_target).await?;
        }

        let extract_target = self.extract_path.join(self.version.as_str());
        if !extract_target.exists() {
            // println!("Extracting MongoDB to {:?}", extract_target);
            extract(&download_target, &extract_target)?;
        }

        let os = get_os()?;
        
        // println!("Starting MongoDB on port {}", self.port);
        MongoProcess::start(&extract_target, self.port, &self.db_path, &os)
    }
}
