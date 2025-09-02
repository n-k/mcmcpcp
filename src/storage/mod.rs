use async_trait::async_trait;

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
    use std::path::PathBuf;
    use directories_next::ProjectDirs;

    let base = if let Some(proj_dirs) = ProjectDirs::from("com", "N K",  "mcmcpcp") {
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
