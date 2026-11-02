use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use serde_json::Value;

use crate::session_keys::SessionKeys;

use super::rpc::RpcSession;

pub(crate) struct AuthTracker {
    connection_id: u64,
    rpc: RpcSession,
    keys: Arc<SessionKeys>,
    pending_tickets: HashMap<u32, [u8; 32]>,
    tickets: HashMap<Vec<u8>, [u8; 32]>,
    pending_joins: HashMap<u32, [u8; 32]>,
}

impl AuthTracker {
    pub(super) fn new(connection_id: u64, keys: Arc<SessionKeys>) -> Self {
        Self {
            connection_id,
            rpc: RpcSession::new(),
            keys,
            pending_tickets: HashMap::new(),
            tickets: HashMap::new(),
            pending_joins: HashMap::new(),
        }
    }

    pub(crate) fn feed(&mut self, from_client: bool, data: &[u8]) -> Result<()> {
        for packet in self.rpc.feed(from_client, data)? {
            let attributes = packet.attributes;
            let token = packet.token;

            let blobs: HashMap<&str, &[u8]> = attributes
                .iter()
                .filter_map(|attribute| {
                    attribute
                        .blob
                        .as_deref()
                        .map(|blob| (attribute.name.as_str(), blob))
                })
                .collect();

            if from_client {
                if let Some(raw) = blobs.get("Param_ClientInfo").copied() {
                    self.pending_tickets.insert(token, client_secret(raw)?);
                }

                if attributes
                    .iter()
                    .any(|attribute| attribute.name.starts_with("Command_RealmJoinRequest_"))
                {
                    let ticket = blobs
                        .get("Param_RealmListTicket")
                        .copied()
                        .context("Realm join ticket is missing")?;

                    let secret = self.tickets.get(ticket).copied().context(
                        "Realm join refers to an unknown ticket; \
                             start capture before WoW login",
                    )?;

                    self.pending_joins.insert(token, secret);
                }
            } else {
                if let Some(ticket) = blobs.get("Param_RealmListTicket").copied() {
                    let secret = self
                        .pending_tickets
                        .remove(&token)
                        .context("Realm list ticket response has no matching client secret")?;

                    self.tickets.insert(ticket.to_vec(), secret);
                }

                if let Some(server_secret) = blobs.get("Param_JoinSecret").copied() {
                    ensure!(
                        server_secret.len() == 32,
                        "Invalid server secret length: {}",
                        server_secret.len(),
                    );

                    let client_secret = self
                        .pending_joins
                        .remove(&token)
                        .context("Realm join response has no matching request")?;

                    let mut key_data = [0u8; 64];
                    key_data[..32].copy_from_slice(&client_secret);
                    key_data[32..].copy_from_slice(server_secret);

                    if self.keys.add_key_data(key_data)? {
                        log::info!("[{}] WoW realm join secrets captured", self.connection_id,);
                    }
                }
            }
        }

        Ok(())
    }
}

fn client_secret(raw: &[u8]) -> Result<[u8; 32]> {
    let end = raw
        .iter()
        .rposition(|byte| *byte != 0)
        .map_or(0, |index| index + 1);

    let raw = &raw[..end];

    let separator = raw
        .iter()
        .position(|byte| *byte == b':')
        .context("ClientInfo JSON prefix separator is missing")?;

    let info: Value =
        serde_json::from_slice(&raw[separator + 1..]).context("Invalid ClientInfo JSON")?;

    let values = info
        .get("info")
        .and_then(|info| info.get("secret"))
        .and_then(Value::as_array)
        .context("ClientInfo secret array is missing")?;

    ensure!(
        values.len() == 32,
        "Invalid client secret length: {}",
        values.len(),
    );

    let mut secret = [0u8; 32];

    for (output, value) in secret.iter_mut().zip(values) {
        let value = value
            .as_u64()
            .context("Client secret contains a non-byte value")?;

        *output = u8::try_from(value).context("Client secret contains a value above 255")?;
    }

    Ok(secret)
}
