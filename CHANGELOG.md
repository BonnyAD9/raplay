# Changelog

## v0.2.0
### New features
- Seeking
- Getting cimestamp and source length
- Gapless playback
- Fade-in/fade-out on play/pause

### API Changes
- Sink is created with its Default implementation
- Move some names closer to root in namespaces
- Custom error type
- Migrate from eyre to anyhow

## v0.1.2
### Bugfixes
- some files would play only first few frames

## v0.1.1
### Bugfixes
- Sink would sometimes choose bad config

## v0.1.0
- Play formats supported by symphonia
- Play sine waves
- Play(Resume)/Pause
- Callback when audio ends
- Callback for errors
- Volume
