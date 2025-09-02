use std::path::PathBuf;
use tokio::fs;
use serde_json::Value;
use anyhow::Result;

use crate::AppSettings;

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

    fn documents_path(&self) -> PathBuf {
        self.base.join("documents.jsonl")
    }
}

#[async_trait::async_trait(?Send)]
impl super::Storage for FileStorage {
    async fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        let json = serde_json::to_string_pretty(settings)?;
        fs::write(self.settings_path(), json).await?;
        Ok(())
    }

    async fn load_settings(&self) -> Result<Option<AppSettings>> {
        match fs::read_to_string(self.settings_path()).await {
            Ok(data) => Ok(Some(serde_json::from_str(&data)?)),
            Err(_) => Ok(None),
        }
    }
}
