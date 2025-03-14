use std::time::Duration;

/// Used to control the playback loop from the sink
#[derive(Clone, Debug)]
pub(super) struct Controls {
    /// Fade duration when play/pause
    pub(super) fade_duration: Duration,
    /// When true, playback plays, when false playback is paused
    pub(super) play: bool,
    /// Sets the volume of the playback
    pub(super) volume: f32,
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
