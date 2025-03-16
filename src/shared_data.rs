use std::{
    fmt::Debug,
    sync::{Mutex, MutexGuard},
};

use crate::{Callback, CallbackInfo, Controls, Error, Result, Source};

/// Data shared between sink and the playback loop
pub struct SharedData {
    /// Used to control the playback loop from the [`Sink`]
    controls: Mutex<Controls>,
    /// The source for the audio
    source: Mutex<Option<Box<dyn Source>>>,
    /// Prefetched source that will play next.
    prefetched: Mutex<Option<Box<dyn Source>>>,
    /// Function used as callback from the playback loop on events
    callback: Callback<CallbackInfo>,
    /// Function used as callback when errors occur on the playback loop
    err_callback: Callback<Error>,
}

impl SharedData {
    /// Creates new shared data
    pub(super) fn new() -> Self {
        Self {
            controls: Mutex::new(Controls::new()),
            source: Mutex::new(None),
            prefetched: Mutex::new(None),
            callback: Callback::default(),
            err_callback: Callback::default(),
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

    pub(super) fn prefeched(
        &self,
    ) -> Result<MutexGuard<'_, Option<Box<dyn Source>>>> {
        Ok(self.prefetched.lock()?)
    }

    /// Invokes callback function
    pub(super) fn invoke_callback(&self, args: CallbackInfo) -> Result<()> {
        self.callback.invoke(args)
    }

    /// Invokes error callback function
    pub(super) fn invoke_err_callback(&self, args: Error) -> Result<()> {
        self.err_callback.invoke(args)
    }

    /// Gets the callback function
    pub(super) fn callback(&self) -> &Callback<CallbackInfo> {
        &self.callback
    }

    /// Gets the error callback function
    pub(super) fn err_callback(&self) -> &Callback<Error> {
        &self.err_callback
    }
}

impl Default for SharedData {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for SharedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedData")
            .field("controls", &self.controls)
            .field("callback", &self.callback)
            .field("err_callback", &self.err_callback)
            .finish()
    }
}
