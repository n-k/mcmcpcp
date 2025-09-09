use async_trait::async_trait;

use crate::{AppSettings, app_settings::Chat};

#[cfg(target_arch = "wasm32")]
mod browser_storage;
#[cfg(not(target_arch = "wasm32"))]
mod file_storage;

#[cfg(not(target_arch = "wasm32"))]
pub type AppStorage = file_storage::FileStorage;
#[cfg(target_arch = "wasm32")]
pub type AppStorage = browser_storage::IdbStorage;

#[async_trait(?Send)]
pub trait Storage {
    async fn save_settings(&self, settings: &AppSettings) -> anyhow::Result<()>;
    async fn load_settings(&self) -> anyhow::Result<Option<AppSettings>>;
    async fn save_chat(&self, chat: &Chat) -> anyhow::Result<u32>;
    async fn list_chats(&self) -> anyhow::Result<Vec<Chat>>;
    async fn get_chat(&self, id: u32) -> anyhow::Result<Option<Chat>>;
    async fn delete_chat(&self, id: u32) -> anyhow::Result<()>;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_storage() -> anyhow::Result<AppStorage> {
    use directories_next::ProjectDirs;
    use std::path::PathBuf;

    let base = if let Some(proj_dirs) = ProjectDirs::from("com", "N K", "mcmcpcp") {
        proj_dirs.config_dir().to_path_buf()
        // Lin: /home/alice/.config/barapp
        // Win: C:\Users\Alice\AppData\Roaming\Foo Corp\Bar App\config
        // Mac: /Users/Alice/Library/Application Support/com.Foo-Corp.Bar-App
    } else {
        PathBuf::from(".")
    };
    let storage = AppStorage::new(base);
    Ok(storage)
}

#[cfg(target_arch = "wasm32")]
pub async fn get_storage() -> anyhow::Result<AppStorage> {
    let storage = AppStorage::new().await?;
    Ok(storage)
}
