use std::sync::Mutex;

use anyhow::{Result, anyhow};

use super::RealmJoinSecrets;

/// Shared by connections belonging to one engine run.
#[derive(Default)]
pub struct SessionStore {
    realm_join_secrets: Mutex<Vec<RealmJoinSecrets>>,
}

impl SessionStore {
    /// Returns true when new key material was inserted.
    pub fn insert_realm_join_secrets(&self, secrets: RealmJoinSecrets) -> Result<bool> {
        let mut entries = self
            .realm_join_secrets
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        if entries.contains(&secrets) {
            return Ok(false);
        }

        entries.push(secrets);

        Ok(true)
    }

    /// Returns a snapshot without retaining the store lock.
    pub fn realm_join_secrets(&self) -> Result<Vec<RealmJoinSecrets>> {
        let entries = self
            .realm_join_secrets
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        Ok(entries.clone())
    }

    pub(crate) fn add_key_data(&self, data: [u8; 64]) -> Result<bool> {
        self.insert_realm_join_secrets(data.into())
    }

    pub fn key_data(&self) -> Result<Vec<[u8; 64]>> {
        let entries = self
            .realm_join_secrets
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        Ok(entries.iter().map(|secrets| *secrets.as_bytes()).collect())
    }
}
