mod auth;
mod framing;
mod generated;
mod messages;
mod methods;
mod rpc;

use std::sync::Arc;

use crate::session_keys::SessionKeys;

pub(crate) use auth::AuthTracker;

pub(crate) struct BnetContext {
    keys: Arc<SessionKeys>,
}

impl BnetContext {
    pub(crate) fn new(keys: Arc<SessionKeys>) -> Self {
        Self { keys }
    }

    pub(crate) fn tracker(&self, connection_id: u64) -> AuthTracker {
        AuthTracker::new(connection_id, self.keys.clone())
    }
}
