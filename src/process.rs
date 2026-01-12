use anyhow::{anyhow, Result};
use std::process::{Child, Command};
use std::path::{Path, PathBuf};
use crate::downloader::Os;

pub struct MongoProcess {
    child: Child,
}

impl MongoProcess {
    pub fn start(
        extracted_path: &Path,
        port: u16,
        db_path: &Path,
        os: &Os,
        bind_ip: &str,
    ) -> Result<Self> {
        let binary_name = match os {
            Os::Windows => "mongod.exe",
            _ => "mongod",
        };

        let binary_path = find_binary(extracted_path, binary_name)
            .ok_or_else(|| anyhow!("Could not find {} in extracted directory", binary_name))?;

        // Ensure binary is executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        if !db_path.exists() {
            std::fs::create_dir_all(db_path)?;
        }

        let child = Command::new(binary_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--dbpath")
            .arg(db_path)
            .arg("--bind_ip")
            .arg(bind_ip)
            .spawn()?;

        Ok(Self { child })
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }
}

fn find_binary(root: &Path, name: &str) -> Option<PathBuf> {
    if root.is_file() {
        if root.file_name()?.to_str()? == name {
            return Some(root.to_path_buf());
        }
        return None;
    }

    if root.is_dir() {
        for entry in std::fs::read_dir(root).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if let Some(found) = find_binary(&path, name) {
                return Some(found);
            }
        }
    }
    None
}
