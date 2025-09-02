use std::path::PathBuf;
use dioxus::logger::tracing::warn;
use tokio::fs;
use anyhow::{bail, Result};

use crate::AppSettings;

#[derive(Debug)]
pub struct FileStorage {
    base: PathBuf,
}

impl FileStorage {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    fn settings_path(&self) -> PathBuf {
        self.base.join("settings.json")
    }

    // fn documents_path(&self) -> PathBuf {
    //     self.base.join("chats")
    // }
    async fn ensure_dir(&self) -> Result<()> {
        let path = self.settings_path();
        let Some(parent) = path.parent() else {
            bail!("Cannot find settings directory");
        };
        tokio::fs::create_dir_all(parent).await?;
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl super::Storage for FileStorage {
    async fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        self.ensure_dir().await?;
        let json = serde_json::to_string_pretty(settings)?;
        let path = self.settings_path();
        fs::write(&path, json).await?;
        warn!("Saved settings to {path:?}");
        Ok(())
    }

    async fn load_settings(&self) -> Result<Option<AppSettings>> {
        self.ensure_dir().await?;
        let path = self.settings_path();
        if !path.exists() {
            return Ok(None);
        }
        match fs::read_to_string(&path).await {
            Ok(data) => Ok(Some(serde_json::from_str(&data)?)),
            Err(_) => Ok(None),
        }
    }
}
