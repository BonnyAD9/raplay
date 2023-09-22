use std::{
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use crate::{err::Result, source::Source, Error};

/// Data shared between sink and the playback loop
pub(super) struct SharedData {
    /// Used to control the playback loop from the [`Sink`]
    controls: Mutex<Controls>,
    /// The source for the audio
    source: Mutex<Option<Box<dyn Source>>>,
    /// Function used as callback from the playback loop on events
    callback: Mutex<Option<Box<dyn FnMut(CallbackInfo) + Send>>>,
    /// Function used as callback when errors occur on the playback loop
    err_callback: Mutex<Option<Box<dyn FnMut(Error) + Send>>>,
}

/// Used to control the playback loop from the sink
#[derive(Clone)]
pub(super) struct Controls {
    /// Fade duration when play/pause
    pub(super) fade_duration: Duration,
    /// When true, playback plays, when false playback is paused
    pub(super) play: bool,
    /// Sets the volume of the playback
    pub(super) volume: f32,
}

/// Callback type and asociated information
#[non_exhaustive]
#[derive(Debug)]
pub enum CallbackInfo {
    /// Invoked when the current source has reached end
    SourceEnded,
    /// Invoked when no sound is playing and you can call hard_pause
    PauseEnds(Instant),
}

impl SharedData {
    /// Creates new shared data
    pub(super) fn new() -> Self {
        Self {
            controls: Mutex::new(Controls::new()),
            source: Mutex::new(None),
            callback: Mutex::new(None),
            err_callback: Mutex::new(None),
        }
    }

    /// Aquires lock on controls
    pub(super) fn controls(&self) -> Result<MutexGuard<'_, Controls>> {
        Ok(self.controls.lock()?)
    }

    /// Aquires lock on source
    pub(super) fn source(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn Source>>>> {
        Ok(self.source.lock()?)
    }

    /// Aquires lock on callback function
    pub(super) fn callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(CallbackInfo) + Send>>>>
    {
        Ok(self.callback.lock()?)
    }

    /// Aquires lock on error callback function
    pub(super) fn err_callback(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn FnMut(Error) + Send>>>> {
        Ok(self.err_callback.lock()?)
    }

    /// Invokes callback function
    pub(super) fn invoke_callback(&self, args: CallbackInfo) -> Result<()> {
        if let Some(cb) = self.callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }

    /// Invokes error callback function
    pub(super) fn invoke_err_callback(&self, args: Error) -> Result<()> {
        if let Some(cb) = self.err_callback()?.as_mut() {
            cb(args)
        }
        Ok(())
    }
}

impl Default for SharedData {
    fn default() -> Self {
        Self::new()
    }
}

impl Controls {
    /// Creates new controls
    pub(super) fn new() -> Self {
        Self {
            fade_duration: Duration::ZERO,
            play: false,
            volume: 1.,
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self::new()
    }
}
