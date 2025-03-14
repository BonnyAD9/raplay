use std::time::Instant;

/// Callback type and asociated information
#[non_exhaustive]
#[derive(Debug)]
pub enum CallbackInfo {
    /// Invoked when the current source has reached end
    SourceEnded,
    /// Invoked when no sound is playing and you can call hard_pause
    PauseEnds(Instant),
}
