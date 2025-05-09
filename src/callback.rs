use std::{
    fmt::Debug,
    mem,
    sync::{Arc, Mutex},
};

use crate::err::Result;

type ArcMutex<T> = Arc<Mutex<T>>;
pub type OptionBox<T> = Option<Box<T>>;

/// Mutexed callback function.
pub struct Callback<T>(ArcMutex<OptionBox<dyn FnMut(T) + Send>>);

impl<T> Callback<T> {
    /// Create new callback function
    pub fn new(f: Option<Box<dyn FnMut(T) + Send>>) -> Self {
        Self(Arc::new(Mutex::new(f)))
    }

    /// Invoke the callback function. It is locked only while it is invoked.
    ///
    /// # Errors
    /// - Callback panicked when called previously on another thread.
    ///
    /// # Panics
    /// - The callback invoked itself.
    pub fn invoke(&self, args: T) -> Result<()> {
        if let Some(cb) = self.0.lock()?.as_mut() {
            cb(args);
        }
        Ok(())
    }

    /// Take the callback and set this to [`None`].
    pub(super) fn take(&self) -> OptionBox<dyn FnMut(T) + Send> {
        self.0.lock().unwrap().take()
    }

    /// Sets new value of the error callback.
    pub(super) fn set(
        &self,
        f: Box<dyn FnMut(T) + Send>,
    ) -> Result<OptionBox<dyn FnMut(T) + Send>> {
        Ok(mem::replace(&mut *self.0.lock()?, Some(f)))
    }
}

impl<T> Default for Callback<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> Debug for Callback<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callback").finish()
    }
}

impl<T> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
