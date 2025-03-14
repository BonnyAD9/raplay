use thiserror::Error;

/// Error type for the symph
#[derive(Error, Debug)]
pub enum Error {
    /// Cannot select track to decode
    #[error("Failed to select a track")]
    CantSelectTrack,
    /// Recoverable error from symphonia
    #[error("Recoverable symphonia error: {0}")]
    SymphRecoverable(symphonia::core::errors::Error),
    /// Error from symphonia
    #[error(transparent)]
    SymphInner(#[from] symphonia::core::errors::Error),
}
