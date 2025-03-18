# Changelog

## v0.4.1
### New features
- Add `Sink::try_load`.

### Changes
- `Sink::load_prefetched` will now never drop the prefetched source. It will be
  either loaded or will stay as prefetch if it fails.

## v0.4.0
### New features
- Add debug implementation to majority of structs.
- Add option to get removed callback functions with `Sink::take_callback` and
  `Sink::take_err_callback`.
- Reexport cpal errors, `SampleFormat`, `FromSample` and `anyhow::Result`.
- `Sink` can restart device and stream with `restart_device` and
  `restart_stream`.
- Add alias for source as `raplay::Source`.
- Allow prefetching of sources with `Sink::prefetch`.
- Allow sending notification when source is about to end with
  `Sink::prefetch_notify`.
- Add option to get current sink device with `Sink::get_device`.
- Add `Sink::get_fade_len`, `Sink::load_prefetched` and
  `Sink::do_prefetch_notify`.

### Breaking changes
- Remove serde as default feature.
- Setters for callback functions now require `Box<dyn FnMut(...) + Send>`
  instead of `Option<impl FnMut(...) + Send + 'static>`.
- Setters of callback functions on sink now return the previous callback
  functions.
- `Sink::set_fade_len` now returns the previous fade length.
- `Sink::set_device` now returns the previous device.
- `Sink::load` now takes `Box<dyn Source>` instead of `impl Source + 'static`.
- Rename `ChannelConverter` and `RateConverter` to `Channel` and `Rate`.
- Move contents of namespaces in `converters::*` directly to `converters`.
- Rename `SineSource` to `Sine`.
- Rename `SymphOptions` to `Options`.
- Move contents of `raplay::callback` directly to `raplay`.
- Move contents of `raplay::err` directly to `raplay`.
- Fully move `raplay::sink` directly to `raplay`.
- Move `raplay::sample_buffer` directly to `raplay`.
- `CallbackInfo::SourceEnded` now also has information on prefetch state.
- When source ends, `SourceEnded` will be sent only once, than will be sent the
  message `NoSource`.

## v0.3.5
### API Changes
- Make serde an optional, default feature
- Sources can now invoke error callback when they encounter recoverable
  errors.

### Bugfixes
- `Symph` source will try to recover from recoverable errors.
- Input devices were selected instead of output devices

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
