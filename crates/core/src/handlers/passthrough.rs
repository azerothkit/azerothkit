use anyhow::Result;

use super::{Handler, HandlerInput, HandlerOutput};

pub(super) struct PassthroughHandler;

impl Handler for PassthroughHandler {
    fn handle(&mut self, input: HandlerInput) -> Result<HandlerOutput> {
        Ok(HandlerOutput::Forward(input.data))
    }
}
