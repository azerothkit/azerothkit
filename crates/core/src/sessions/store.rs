use std::sync::Mutex;

use anyhow::{Result, anyhow};

use super::{RealmJoinSecrets, WorldSessionKey};

#[derive(Default)]
struct StoreState {
    realm_join_secrets: Vec<RealmJoinSecrets>,
    world_candidates: Vec<WorldSessionKey>,
    world_sessions: Vec<WorldSessionKey>,
}

#[derive(Default)]
pub struct SessionStore {
    state: Mutex<StoreState>,
}

impl SessionStore {
    pub fn insert_realm_join_secrets(
        &self,
        secrets: RealmJoinSecrets,
    ) -> Result<bool> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        if state.realm_join_secrets.contains(&secrets) {
            return Ok(false);
        }

        state.realm_join_secrets.push(secrets);
        Ok(true)
    }

    pub fn realm_join_secrets(
        &self,
    ) -> Result<Vec<RealmJoinSecrets>> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        Ok(state.realm_join_secrets.clone())
    }

    pub(crate) fn add_key_data(
        &self,
        data: [u8; 64],
    ) -> Result<bool> {
        self.insert_realm_join_secrets(data.into())
    }

    pub fn key_data(&self) -> Result<Vec<[u8; 64]>> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        Ok(state
            .realm_join_secrets
            .iter()
            .map(|secrets| *secrets.as_bytes())
            .collect())
    }

    pub(crate) fn add_world_candidates(
        &self,
        keys: &[WorldSessionKey],
    ) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        for key in keys {
            if !state.world_sessions.contains(key)
                && !state.world_candidates.contains(key)
            {
                state.world_candidates.push(*key);
            }
        }

        Ok(())
    }

    pub(crate) fn world_session_candidates(
        &self,
    ) -> Result<Vec<WorldSessionKey>> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        Ok(state
            .world_sessions
            .iter()
            .chain(state.world_candidates.iter())
            .copied()
            .collect())
    }

    pub(crate) fn confirm_world_session(
        &self,
        key: WorldSessionKey,
    ) -> Result<bool> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("Session store is poisoned"))?;

        state.world_candidates.retain(|candidate| *candidate != key);

        if state.world_sessions.contains(&key) {
            return Ok(false);
        }

        state.world_sessions.push(key);
        Ok(true)
    }
}
