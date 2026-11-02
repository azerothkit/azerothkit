use anyhow::Result;

use super::{Direction, HandlerInput, HandlerOutput};

/// One instance per connection, handling both directions.
/// Calls are serialized through exclusive mutable access.
pub(crate) trait Handler: Send {
    /// Receives decrypted data when TLS interception is enabled.
    ///
    /// An error is fatal to the connection.
    /// Observers must handle nonfatal inspection errors internally.
    fn handle(&mut self, input: HandlerInput) -> Result<HandlerOutput>;

    /// Called when a TCP direction reaches EOF.
    /// Output must be sent before shutting down the corresponding writer.
    fn finish(&mut self, _direction: Direction) -> Result<HandlerOutput> {
        Ok(HandlerOutput::None)
    }
}
