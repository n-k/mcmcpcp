use anyhow::{Result, bail};
use dioxus::logger::tracing::warn;
use std::path::PathBuf;
use tokio::fs;

use crate::AppSettings;
use crate::app_settings::Chat;

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

    fn chats_path(&self) -> PathBuf {
        self.base.join("chats")
    }

    async fn ensure_dir(&self) -> Result<()> {
        let path = self.settings_path();
        let Some(parent) = path.parent() else {
            bail!("Cannot find settings directory");
        };
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    async fn get_next_chat_id(&self) -> Result<u32> {
        self.ensure_dir().await?;
        let chats_dir = self.chats_path();
        if !chats_dir.exists() {
            tokio::fs::create_dir_all(&chats_dir).await?;
        }
        let mut entries = tokio::fs::read_dir(&chats_dir).await?;
        let mut idx: u32 = 0;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let Some(n) = path.file_name() else {
                    continue;
                };
                let Some(n) = n.to_str() else {
                    continue;
                };
                let mut n = n.to_lowercase();
                if n.ends_with(".json") && n.len() > 5 {
                    let _ = n.split_off(n.len() - 5);
                    if let Ok(i) = n.parse::<u32>() {
                        if idx < i {
                            idx = i;
                        }
                    }
                }
            }
        }
        Ok(idx + 1)
    }
}

#[async_trait::async_trait(?Send)]
impl super::Storage for FileStorage {
    async fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        self.ensure_dir().await?;
        let json = serde_json::to_string_pretty(settings)?;
        let path = self.settings_path();
        fs::write(&path, json).await?;
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

    async fn save_chat(&self, chat: &Chat) -> anyhow::Result<u32> {
        let file_idx = if let Some(id) = &chat.id {
            *id
        } else {
            self.get_next_chat_id().await?
        };
        let file_name = format!("{file_idx}.json");
        let path = self.chats_path().join(file_name);
        let mut c = chat.clone();
        c.id = Some(file_idx);
        let json = serde_json::to_string_pretty(&c)?;
        fs::write(&path, json).await?;
        Ok(file_idx)
    }

    async fn list_chats(&self) -> anyhow::Result<Vec<Chat>> {
        let chats_dir = self.chats_path();
        if !chats_dir.exists() {
            return Ok(vec![]);
        }
        let mut entries = tokio::fs::read_dir(&chats_dir).await?;
        let mut chats = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        match tokio::fs::read_to_string(&path).await {
                            Ok(content) => match serde_json::from_str::<Chat>(&content) {
                                Ok(chat) => chats.push(chat),
                                Err(e) => warn!("Failed to parse chat from {path:?}: {e}"),
                            },
                            Err(e) => warn!("Failed to read chat file {path:?}: {e}"),
                        }
                    }
                }
            }
        }
        Ok(chats)
    }

    async fn get_chat(&self, id: u32) -> anyhow::Result<Option<Chat>> {
        let file_name = format!("{id}.json");
        let path = self.chats_path().join(file_name);
        if !path.exists() {
            return Ok(None);
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match serde_json::from_str::<Chat>(&content) {
                Ok(chat) => Ok(Some(chat)),
                Err(e) => {
                    warn!("Failed to parse chat from {path:?}: {e}");
                    Ok(None)
                }
            },
            Err(e) => {
                warn!("Failed to read chat file {path:?}: {e}");
                Ok(None)
            }
        }
    }

    async fn delete_chat(&self, id: u32) -> anyhow::Result<()> {
        let file_name = format!("{id}.json");
        let path = self.chats_path().join(file_name);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }
}
