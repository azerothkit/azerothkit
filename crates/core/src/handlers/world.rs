use std::sync::Arc;

use anyhow::{Context, Result};

use crate::protocol::world::{WorldCodec, WorldEvent};
use crate::sessions::SessionStore;

use super::{Direction, Handler, HandlerInput, HandlerOutput};
use crate::features::WorldPipeline;

pub(super) struct WorldHandler {
    connection_id: u64,
    sessions: Arc<SessionStore>,
    codec: WorldCodec,
    features: WorldPipeline,
    encryption_announced: bool,
}

impl WorldHandler {
    pub(super) fn new(
        connection_id: u64,
        sessions: Arc<SessionStore>,
        features: WorldPipeline,
    ) -> Self {
        Self {
            connection_id,
            sessions,
            codec: WorldCodec::default(),
            features,
            encryption_announced: false,
        }
    }
}

impl Handler for WorldHandler {
    fn handle(&mut self, input: HandlerInput) -> Result<HandlerOutput> {
        let from_client = input.direction.is_from_client();

        self.codec.push(from_client, &input.data);

        let mut output = Vec::new();

        while let Some(event) = self
            .codec
            .next(from_client)
            .context("World decode failed")?
        {
            match event {
                WorldEvent::Greeting(wire) => {
                    output.push(wire);
                }

                WorldEvent::Packet(frame) => {
                    if let Some(request) = &frame.auth {
                        let secrets = self.sessions.realm_join_secrets()?;
                        let sessions = self.sessions.world_session_candidates()?;

                        let candidates = request.resolve(&secrets, &sessions)?;

                        let keys = candidates
                            .iter()
                            .map(|candidate| candidate.session())
                            .collect::<Vec<_>>();

                        if request.is_initial() {
                            self.sessions.add_world_candidates(&keys)?;
                        } else {
                            // A valid continued-session proof authenticates
                            // the parent session key before GCM starts.
                            for key in keys {
                                self.sessions.confirm_world_session(key)?;
                            }
                        }

                        self.codec.bind_candidates(candidates)?;
                    }

                    if let Some(session) = frame.verified_session {
                        self.sessions.confirm_world_session(session)?;

                        if !self.encryption_announced {
                            self.encryption_announced = true;

                            log::info!("[{}] World encryption verified", self.connection_id,);
                        }
                    }

                    let rewritten = if frame.encrypted {
                        self.features
                            .process(input.direction, &frame.packets)
                            .context("World feature processing failed")?
                    } else {
                        None
                    };

                    let packets = rewritten.as_deref().unwrap_or(frame.packets.as_slice());

                    output.push(
                        self.codec
                            .encode(from_client, &frame, packets)
                            .context("World encode failed")?,
                    );
                }
            }
        }

        match output.len() {
            0 => Ok(HandlerOutput::None),
            1 => Ok(HandlerOutput::Forward(
                output.pop().expect("one output chunk"),
            )),
            _ => Ok(HandlerOutput::Batch(output)),
        }
    }

    fn finish(&mut self, direction: Direction) -> Result<HandlerOutput> {
        self.codec
            .finish(direction.is_from_client())
            .context("Incomplete World stream at EOF")?;

        Ok(HandlerOutput::None)
    }
}
