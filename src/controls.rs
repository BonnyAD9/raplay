use std::time::Duration;

/// Used to control the playback loop from the sink
#[derive(Clone, Debug)]
pub(super) struct Controls {
    /// Fade duration when play/pause
    pub(super) fade_duration: Duration,
    /// How long before source end should we send the prefetch notify callback.
    /// Zero means don't send notify prefetch.
    pub(super) prefetch: Duration,
    /// Sets the volume of the playback
    pub(super) volume: f32,
    /// When true, playback plays, when false playback is paused
    pub(super) play: bool,
}

impl Controls {
    /// Creates new controls
    pub(super) fn new() -> Self {
        Self {
            fade_duration: Duration::ZERO,
            prefetch: Duration::ZERO,
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
