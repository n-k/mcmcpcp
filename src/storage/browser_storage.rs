use anyhow::anyhow;
use async_trait::async_trait;
use idb::{Database, DatabaseEvent, Factory, KeyPath, ObjectStoreParams, TransactionMode};
use js_sys::wasm_bindgen::JsValue;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_wasm_bindgen::Serializer;

use dioxus::logger::tracing::warn;

use crate::AppSettings;
use crate::storage::Chat;
use super::Storage;

#[derive(Debug)]
pub struct IdbStorage {
    db: Database,
}

impl IdbStorage {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Self::create_db().await?;
        Ok(Self { db })
    }

    pub async fn create_db() -> anyhow::Result<Database> {
        // Get a factory instance from global scope
        let factory = Factory::new().map_err(|e| anyhow!("{e:?}"))?;

        // Create an open request for the database
        let mut open_request = factory
            .open("app_storage", Some(1))
            .map_err(|e| anyhow!("{e:?}"))?;

        // Add an upgrade handler for database
        open_request.on_upgrade_needed(|event| {
            // Get database instance from event
            let database = event.database().unwrap();

            // Prepare object store params
            let mut store_params = ObjectStoreParams::new();
            store_params.auto_increment(false);
            store_params.key_path(Some(KeyPath::new_single("id")));
            let _store = database
                .create_object_store("settings", store_params.clone())
                .unwrap();
            let mut store_params = ObjectStoreParams::new();
            store_params.auto_increment(true);
            store_params.key_path(Some(KeyPath::new_single("id")));
            let _store = database
                .create_object_store("sessions", store_params)
                .unwrap();
        });

        // `await` open request
        let db = open_request.await.map_err(|e| anyhow!("{e:?}"))?;
        Ok(db)
    }
}

#[async_trait::async_trait(?Send)]
impl Storage for IdbStorage {
    async fn save_settings(&self, settings: &AppSettings) -> anyhow::Result<()> {
        // let db = IdbStorage::create_db().await?;
        // warn!("Starting save process...");
        let transaction = self.db
            .transaction(&["settings"], TransactionMode::ReadWrite)
            .map_err(|e| anyhow!("{e:?}"))?;
        let store = transaction
            .object_store("settings")
            .map_err(|e| anyhow!("{e:?}"))?;

        // warn!("Got store, will put");
        let doc = settings.serialize(&Serializer::json_compatible()).unwrap();
        // warn!("serialized: {doc:?}");
        let put_res = store
            .put(
                &doc,
                None,
            )
            .map_err(|e| anyhow!("{e:?}"))?
            .await
            .map_err(|e| anyhow!("{e:?}"))?;
        // warn!("put op: {put_res:?}");
        transaction.commit().unwrap().await.unwrap();
        // warn!("done");
        Ok(())
    }

    async fn load_settings(&self) -> anyhow::Result<Option<AppSettings>> {
        // let db = IdbStorage::create_db().await?;
        let transaction = self.db
            .transaction(&["settings"], TransactionMode::ReadOnly)
            .map_err(|e| anyhow!("{e:?}"))?;
        let store = transaction.object_store("settings").unwrap();
        let stored_settings: Option<JsValue> = store
            .get(JsValue::from_f64(1.))
            .map_err(|e| anyhow!("{e:?}"))?
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        // Deserialize the stored data
        let stored_settings: Option<anyhow::Result<AppSettings>> =
            stored_settings.map(|stored_settings| {
                serde_wasm_bindgen::from_value(stored_settings).map_err(|e| anyhow!("{e:?}"))
            });
        let stored_settings = stored_settings.transpose()?;

        // Wait for the transaction to complete (alternatively, you can also commit the transaction)
        transaction.await.map_err(|e| anyhow!("{e:?}"))?;
        Ok(stored_settings)
    }

    async fn save_chat(&self, chat: &Chat) -> anyhow::Result<u32> {
        let transaction = self.db
            .transaction(&["sessions"], TransactionMode::ReadWrite)
            .map_err(|e| anyhow!("{e:?}"))?;
        let store = transaction
            .object_store("sessions")
            .map_err(|e| anyhow!("{e:?}"))?;

        let doc = chat.serialize(&Serializer::json_compatible()).unwrap();
        // warn!("serialized: {doc:?}");
        let put_res = if chat.id.is_some() {
            warn!("putting...");
            store
                .put(
                    &doc,
                    None,
                )
                .map_err(|e| anyhow!("{e:?}"))?
                .await
                .map_err(|e| anyhow!("{e:?}"))?
        } else {
            warn!("adding...");
            store
                .add(
                    &doc,
                    None,
                )
                .map_err(|e| anyhow!("{e:?}"))?
                .await
                .map_err(|e| anyhow!("{e:?}"))?
        };
        warn!("put op: {put_res:?}");
        transaction.commit()
            .map_err(|e| anyhow!("{e:?}"))?
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(put_res.as_f64().map(|n| n as u32).unwrap())
    }
    
    async fn list_chats(&self) -> anyhow::Result<Vec<Chat>> {
        let transaction = self.db
            .transaction(&["sessions"], TransactionMode::ReadWrite)
            .map_err(|e| anyhow!("{e:?}"))?;
        let store = transaction
            .object_store("sessions")
            .map_err(|e| anyhow!("{e:?}"))?;

        let all = store.get_all(None, None).unwrap().await.unwrap();
        let mut all_chats: Vec<Chat> = vec![];
        for v in all {
            let c = serde_wasm_bindgen::from_value(v).map_err(|e| anyhow!("{e:?}"))?;
            all_chats.push(c);
        }

        transaction.await.map_err(|e| anyhow!("{e:?}"))?;
        Ok(all_chats)
    }
    
    async fn get_chat(&self, id: u32) -> anyhow::Result<Option<Chat>> {
        let transaction = self.db
            .transaction(&["sessions"], TransactionMode::ReadWrite)
            .map_err(|e| anyhow!("{e:?}"))?;
        let store = transaction
            .object_store("sessions")
            .map_err(|e| anyhow!("{e:?}"))?;

        let stored_chat: Option<JsValue> = store
            .get(JsValue::from_f64(id.into()))
            .map_err(|e| anyhow!("{e:?}"))?
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        // Deserialize the stored data
        let stored_chat: Option<anyhow::Result<Chat>> =
            stored_chat.map(|stored_chat| {
                serde_wasm_bindgen::from_value(stored_chat).map_err(|e| anyhow!("{e:?}"))
            });
        let stored_chat = stored_chat.transpose()?;

        transaction.await.map_err(|e| anyhow!("{e:?}"))?;
        Ok(stored_chat)
    }
    
    async fn delete_chat(&self, id: u32) -> anyhow::Result<()> {
        let transaction = self.db
            .transaction(&["sessions"], TransactionMode::ReadWrite)
            .map_err(|e| anyhow!("{e:?}"))?;
        let store = transaction
            .object_store("sessions")
            .map_err(|e| anyhow!("{e:?}"))?;

        store.delete(JsValue::from_f64(id.into())).unwrap().await.map_err(|e| anyhow!("{e:?}"))?;

        transaction.await.map_err(|e| anyhow!("{e:?}"))?;
        Ok(())
    }
}
