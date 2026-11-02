use anyhow::Result;

use crate::protocol::bnet::AuthTracker;

use super::{Handler, HandlerInput, HandlerOutput};

pub(super) struct BnetHandler {
    connection_id: u64,
    tracker: Option<AuthTracker>,
}

impl BnetHandler {
    pub(super) fn new(connection_id: u64, tracker: AuthTracker) -> Self {
        Self {
            connection_id,
            tracker: Some(tracker),
        }
    }
}

impl Handler for BnetHandler {
    fn handle(&mut self, input: HandlerInput) -> Result<HandlerOutput> {
        if let Some(tracker) = self.tracker.as_mut() {
            if let Err(error) = tracker.feed(input.direction.is_from_client(), &input.data) {
                log::warn!(
                    "[{}] Battle.net inspection disabled; \
                     forwarding unchanged: {error:#}",
                    self.connection_id,
                );

                self.tracker = None;
            }
        }

        Ok(HandlerOutput::Forward(input.data))
    }
}
