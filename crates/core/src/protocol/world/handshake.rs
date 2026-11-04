use anyhow::{Context, Result, ensure};

use crate::sessions::{RealmJoinSecrets, WorldSessionKey};

use super::crypto::{KeyCandidate, derive_session, verify_continued};
use super::{WorldOpcodes, WorldPacket};

pub enum AuthRequest {
    Initial {
        server: [u8; 32],
        client: [u8; 32],
    },
    Continued {
        server: [u8; 32],
        client: [u8; 32],
        connection_key: [u8; 8],
        proof: [u8; 24],
    },
}

impl AuthRequest {
    pub fn is_initial(&self) -> bool {
        matches!(self, Self::Initial { .. })
    }

    pub fn resolve(
        &self,
        secrets: &[RealmJoinSecrets],
        sessions: &[WorldSessionKey],
    ) -> Result<Vec<KeyCandidate>> {
        let candidates = match self {
            Self::Initial { server, client } => secrets
                .iter()
                .map(|secrets| {
                    let session = derive_session(secrets, server, client);
                    KeyCandidate::new(session, server, client)
                })
                .collect::<Vec<_>>(),

            Self::Continued {
                server,
                client,
                connection_key,
                proof,
            } => sessions
                .iter()
                .filter(|session| verify_continued(session, server, client, connection_key, proof))
                .map(|session| KeyCandidate::new(*session, server, client))
                .collect::<Vec<_>>(),
        };

        ensure!(
            !candidates.is_empty(),
            "No matching World session secrets; \
             capture Battle.net login and the initial World connection",
        );

        Ok(candidates)
    }
}

pub(super) struct Handshake {
    opcodes: WorldOpcodes,
    server: Option<[u8; 32]>,
    auth_seen: bool,
    armed: bool,
    encrypted: bool,
}

impl Handshake {
    pub fn new(opcodes: WorldOpcodes) -> Self {
        Self {
            opcodes,
            server: None,
            auth_seen: false,
            armed: false,
            encrypted: false,
        }
    }

    pub fn encrypted(&self) -> bool {
        self.encrypted
    }

    pub fn observe(
        &mut self,
        from_client: bool,
        packet: &WorldPacket,
    ) -> Result<Option<AuthRequest>> {
        let payload = &packet.payload;

        match (from_client, packet.opcode) {
            (false, opcode) if opcode == self.opcodes.auth_challenge => {
                ensure!(
                    payload.len() == 65,
                    "Unsupported World AuthChallenge size: {}",
                    payload.len(),
                );

                ensure!(
                    self.server.is_none() && !self.auth_seen,
                    "Duplicate World AuthChallenge",
                );

                self.server = Some(payload[32..64].try_into()?);
            }

            (true, opcode)
            if opcode == self.opcodes.auth_session
                || opcode == self.opcodes.auth_continued =>
                {
                    ensure!(
                    !self.auth_seen,
                    "Duplicate World authentication",
                );

                    let server = self.server
                        .context("World AuthChallenge is missing")?;

                    let request =
                        if packet.opcode == self.opcodes.auth_session {
                            ensure!(
                            payload.len() >= 76,
                            "Short World AuthSession",
                        );

                            AuthRequest::Initial {
                                server,
                                client: payload[20..52].try_into()?,
                            }
                        } else {
                            ensure!(
                            payload.len() >= 72,
                            "Short World AuthContinuedSession",
                        );

                            AuthRequest::Continued {
                                server,
                                client: payload[8..40].try_into()?,
                                proof: payload[40..64].try_into()?,
                                connection_key: payload[64..72].try_into()?,
                            }
                        };

                    self.auth_seen = true;

                    return Ok(Some(request));
                }

            (false, opcode) if opcode == self.opcodes.enter_encrypted => {
                ensure!(
                    self.auth_seen,
                    "World authentication is missing",
                );

                ensure!(
                    payload.len() == 69,
                    "Unsupported EnterEncryptedMode size: {}",
                    payload.len(),
                );

                self.armed = payload[68] & 0x80 != 0;
            }

            (true, opcode) if opcode == self.opcodes.encrypted_ack => {
                ensure!(
                    self.armed,
                    "World encryption ACK without EnterEncryptedMode",
                );

                self.encrypted = true;
            }

            _ => {}
        }

        Ok(None)
    }
}