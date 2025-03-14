use thiserror::Error;

use crate::source::symph;

pub use cpal::{
    BuildStreamError, DefaultStreamConfigError, DevicesError,
    PauseStreamError, PlayStreamError, StreamError,
    SupportedStreamConfigsError,
};

/// Result with this crate error type [`enum@Error`]
pub type Result<T> = std::result::Result<T, Error>;

/// Error type of this crate
#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to convert to timestamp")]
    CannotDetermineTimestamp,
    /// Error that is returned when something fails to lock some resource
    #[error("Failed to aquire lock")]
    PoisonError,
    /// Returned when the device uses an unsupported sample format
    #[error("Format supported by the device is not supported by the library")]
    UnsupportedSampleFormat,
    /// Returned when the sink fails to select output device
    #[error("No available output device was found")]
    NoOutDevice,
    /// Returned when some feature is not supported
    #[error("{component} doesn't support {feature}")]
    Unsupported {
        component: &'static str,
        feature: &'static str,
    },
    /// Returned when Sink tries to do action on Source, but there is no source
    #[error("Cannot operate on a source because there is no source playing")]
    NoSourceIsPlaying,
    /// Cpal errors
    #[error(transparent)]
    Cpal(#[from] CpalError),
    /// Errors from the [`crate::source::Symph`] source
    #[error(transparent)]
    Symph(#[from] symph::Error),
    /// Any other error, usually from a custom source
    #[error(transparent)]
    Other(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        if value.is::<Self>() {
            value.downcast().unwrap()
        } else {
            Self::Other(value)
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_value: std::sync::PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

macro_rules! impl_cpal {
    ($($i:ident -> $t:ty),+ $(,)?) => {
        $(
            impl From<$t> for Error {
                fn from(value: $t) -> Self {
                    Self::Cpal(value.into())
                }
            }
        )+

        #[derive(Error, Debug)]
        pub enum CpalError {
            $(
                #[error(transparent)]
                $i(#[from] $t),
            )+
        }
    };
}

impl_cpal!(
    DefaultStreamConfig -> DefaultStreamConfigError,
    Stream -> StreamError,
    BuildStream -> BuildStreamError,
    PlayStream -> PlayStreamError,
    SupportedConfigs -> SupportedStreamConfigsError,
    PauseStreamError -> PauseStreamError,
    DevicesError -> DevicesError,
);
