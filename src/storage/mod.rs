use async_trait::async_trait;
use serde_json::Value;

use crate::AppSettings;

#[cfg(target_arch = "wasm32")]
mod browser_storage;
#[cfg(not(target_arch = "wasm32"))]
mod file_storage;

#[cfg(not(target_arch = "wasm32"))]
type AppStorage = file_storage::FileStorage;
#[cfg(target_arch = "wasm32")]
type AppStorage = browser_storage::IdbStorage;


#[async_trait(?Send)]
pub trait Storage {
    async fn save_settings(&self, settings: &AppSettings) -> anyhow::Result<()>;
    async fn load_settings(&self) -> anyhow::Result<Option<AppSettings>>;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_storage() -> anyhow::Result<AppStorage> {
    let storage = AppStorage::new("./data");
    Ok(storage)
}

#[cfg(target_arch = "wasm32")]
pub async fn get_storage() -> anyhow::Result<AppStorage> {
    let storage = AppStorage::new().await?;
    Ok(storage)
}
