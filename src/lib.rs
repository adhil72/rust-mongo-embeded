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
    SettingUpUser,
    VerifyingCredentials,
    DBInitialized,
}

pub struct MongoEmbedded {
    pub version: String,
    pub download_path: PathBuf,
    pub extract_path: PathBuf,
    pub db_path: PathBuf,
    pub port: u16,
    pub bind_ip: String,
    pub username: Option<String>,
    pub password: Option<String>,
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
            username: None,
            password: None,
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

    pub fn set_credentials(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
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
        
        // Calculate initial connection string for readiness check
        let uri = if self.bind_ip.contains('/') || self.bind_ip.ends_with(".sock") {
            // Assume unix socket
            // Minimal URL encoding for path, replacing / with %2F. 
            // This is required for the rust mongodb driver to recognize it as a socket.
            // Note: When using sockets with mongodb crate, we often need to ensure the host is just the encoded path.
            let encoded = self.bind_ip.replace("/", "%2F");
            format!("mongodb://{}/?directConnection=true", encoded)
        } else {
            format!("mongodb://{}:{}/?directConnection=true", self.bind_ip, self.port)
        };

        // Start process with auth flag if credentials are requested
        let auth_enabled = self.username.is_some() && self.password.is_some();
        let mut process = MongoProcess::start(&extract_target, self.port, &self.db_path, &os, &self.bind_ip, auth_enabled, uri.clone())?;
        
        // Need to wait for it to be ready
        // We can try to connect
        let mut client_options = mongodb::options::ClientOptions::parse(&uri).await?;
        client_options.connect_timeout = Some(std::time::Duration::from_secs(2));
        client_options.server_selection_timeout = Some(std::time::Duration::from_secs(2));

        // Simple loop to wait for readiness
        let mut connected = false;
        let start = std::time::Instant::now();
        println!("DEBUG: Waiting for MongoDB to start at {}", uri);
        while start.elapsed() < std::time::Duration::from_secs(30) {
            let client = mongodb::Client::with_options(client_options.clone())?;
            match client.list_database_names(None, None).await {
                Ok(_) => {
                    connected = true;
                    break;
                }
                Err(e) => {
                    println!("DEBUG: Connection attempt failed: {:?}", e);
                    // If unauthorized error, it means we are connected but need auth, which is fine for readiness check
                    // "Unauthorized" usually is error code 13
                    match *e.kind {
                         mongodb::error::ErrorKind::Command(ref cmd_err) => {
                             if cmd_err.code == 51 || cmd_err.code == 13 || cmd_err.code == 18 { // 51: UserAlreadyExists?, 13: Unauthorized, 18: AuthFailed
                                 connected = true;
                                 break;
                             }
                         },
                         _ => {}
                    }
                }
            }
             
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        if !connected {
             println!("DEBUG: Timed out waiting for start.");
             process.kill()?;
             return Err(anyhow::anyhow!("Timed out waiting for MongoDB to start"));
        }

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
             callback(InitStatus::SettingUpUser);
             let client = mongodb::Client::with_options(client_options.clone())?;

             // Try to create user. This only works if localhost exception is active (no users)
             use mongodb::bson::doc;
             let db = client.database("admin");
             let run_cmd = db.run_command(doc! {
                "createUser": username,
                "pwd": password,
                "roles": [
                    { "role": "root", "db": "admin" }
                ]
             }, None).await;

             match run_cmd {
                Ok(_) => {
                    // Created user successfully
                },
                Err(e) => {
                     // Check if error is unauthorized or "already exists"
                     let kind = &*e.kind;
                     let needs_verify;
                     if let mongodb::error::ErrorKind::Command(cmd_err) = kind {
                         if cmd_err.code == 51 { // UserAlreadyExists
                             needs_verify = true;
                         } else if cmd_err.code == 13 { // Unauthorized
                             needs_verify = true;
                         } else {
                             // Unexpected error, maybe fail or try verify
                             needs_verify = true; 
                         }
                     } else {
                         needs_verify = true; // Connection error or other?
                     }

                     if needs_verify {
                         callback(InitStatus::VerifyingCredentials);
                         // Try to authenticate
                         let mut auth_opts = client_options.clone();
                         auth_opts.credential = Some(mongodb::options::Credential::builder()
                            .username(username.clone())
                            .password(password.clone())
                            .source("admin".to_string())
                            .build());
                         
                         let auth_client = mongodb::Client::with_options(auth_opts)?;
                         // Verify by running a command that requires auth
                         if let Err(auth_err) = auth_client.database("admin").run_command(doc! { "ping": 1 }, None).await {
                             process.kill()?;
                             return Err(anyhow::anyhow!("Authentication failed or invalid credentials provided: {}", auth_err));
                         }
                     }
                }
             }

             // Update connection string to include credentials
             let final_uri;
             if self.bind_ip.contains('/') || self.bind_ip.ends_with(".sock") {
                 let encoded = self.bind_ip.replace("/", "%2F");
                 // For sockets, credentials go in the beginning
                 final_uri = format!("mongodb://{}:{}@{}", username, password, encoded);
             } else {
                 final_uri = format!("mongodb://{}:{}@{}:{}/", username, password, self.bind_ip, self.port);
             }
             process.connection_string = final_uri;
        }

        callback(InitStatus::DBInitialized);
        Ok(process)
    }
}

