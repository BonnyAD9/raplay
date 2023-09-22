# Changelog

## v0.3.4
### Bugfixes
- PauseEnds was called after load

## v0.3.3
### Bugfixes
- Make hard pause callback return time

## v0.3.2
### Bugfixes
- Proper visibility

## v0.3.1
### New features
- Add serialize and deserialize to timestamp

## v0.3.0
### New features
- Message when pause ends
- Sink implements Debug
- Get timestamp when seeking
- Add option to seek by
- Option to set buffer size
- Option to get device info
- Option to select output device

### Bugfixes
- Fix typo in source trait function name

## v0.2.2
### Bugfixes
- Symph now returns correct timestamps right after seeking

## v0.2.1
### Bugfixes
- Symph was mot using `err::Error` in pulic api

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
