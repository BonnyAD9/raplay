use thiserror::Error;

use crate::source::symph;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to aquire lock")]
    PoisonError,
    #[error("Format supported by the device is not supported by the library")]
    UnsupportedSampleFormat,
    #[error("No available output device was found")]
    NoOutDevice,
    #[error(transparent)]
    Cpal(#[from] CpalError),
    #[error(transparent)]
    Symph(#[from] symph::Error),
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
    DefaultStreamConfig -> cpal::DefaultStreamConfigError,
    Stream -> cpal::StreamError,
    BuildStream -> cpal::BuildStreamError,
    PlayStream -> cpal::PlayStreamError,
    SupportedConfigs -> cpal::SupportedStreamConfigsError,
);
