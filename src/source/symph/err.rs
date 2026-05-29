use thiserror::Error;

/// Error type for the symph
#[derive(Error, Debug)]
pub enum Error {
    /// Cannot select track to decode
    #[error("Failed to select a track")]
    CantSelectTrack,
    /// Cannot convert duration to the symhonia primitive because it contains
    /// too large value.
    #[error("Duration contains too large value.")]
    TooLargeDuration,
    /// Recoverable error from symphonia
    #[error("Recoverable symphonia error: {0}")]
    SymphRecoverable(symphonia::core::errors::Error),
    /// Error from symphonia
    #[error(transparent)]
    SymphInner(#[from] symphonia::core::errors::Error),
}

impl Error {
    pub fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }
}
