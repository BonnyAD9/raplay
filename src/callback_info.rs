use std::time::{Duration, Instant};

use crate::PrefetchState;

/// Callback type and asociated information
#[non_exhaustive]
#[derive(Debug)]
pub enum CallbackInfo {
    /// Invoked when the current source has reached end. Parameter specifies
    /// status of prefetch. [`None`] means no prefetch was set, `true` means
    /// successfull prefetch, `false` means that prefetch coululdn't be set
    /// because the configuration didn't match.
    SourceEnded(PrefetchState),
    /// No source is available to play.
    NoSource,
    /// Invoked when no sound is playing and you can call hard_pause
    PauseEnds(Instant),
    /// Prefetch time triggered. Only the given remaining playback time
    /// remains.
    PrefetchTime(Duration),
}
