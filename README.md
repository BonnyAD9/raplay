# raplay
Library for playing audio.

The library is not tested systematically, but I have been using daily in my
music player for years and I'm not aware of any unexpected problems.

## Features
- Play(Resume)/Pause
- Callback when audio ends
- Callback for errors
- Volume
- Seeking
- Get audio position and length
- Fade-in/fade-out on play/pause
- Gapless playback with prefetching of the next source.

## Supported formats
All the decoding is done by
[symphonia](https://github.com/pdeljanov/Symphonia/tree/master), so the
supported formats are the same as symphonia. But you can implement your own
source if you want to.

## Examples

### Play a sine wave
```rust
use raplay::{Sink, source::Sine};

let mut sink = Sink::default(); // Get the default output
let src = Sine::new(1000.); // Create 1000Hz sine source
sink.load(Box::new(src), true)?; // Play the sine wave
# Ok::<(), raplay::Error>(())
```

### Play a mp3 file
```rust
use std::fs::File;
use raplay::{Sink, source::Symph};

let mut sink = Sink::default(); // Get the default output
let file = File::open("music.mp3").unwrap(); // Open the mp3 file
let src = Symph::try_new(file, &Default::default())?; // Create a symphonia
                                                      // decoder source
sink.load(Box::new(src), true); // Play the mp3 file
# Ok::<(), raplay::Error>(())
```

## Feature flags
- `serde`: Implements `Serialize` and `Deserialize` for `Timestamp`.

The following features determine the audio backend. They directly correspond to
features in [cpal](https://github.com/RustAudio/cpal#optional-features):
- `asio` (Windows): ASIO backkend for low-latency audio, bypassing the Windows
  audio stack. Requires ASIO drivers and LLVM/Clang.
- `audioworklet` (WebAssembly): Audio Worklet backend for lower latency web
  audio. Requires atomics support
  (`RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals"`) and
  `Cross-Origin` headers for `SharedArrayBuffer`.
- `jack` (Linux, BSD, macOS, Windows): JACK Audio Connection Kit backend for
  pro-audio routing and inter-application connectivity. Requires
  `libjack-jackd2-dev` (Debian/Ubuntu) or `jack-devel` (Fedora).
- `pipewire` (Linux, BSD): PipeWire media server backend. Requires
  `libpipewire-0.3-dev` (Debian/Ubuntu) or `pipewire-devel` (Fedora).
- `pulseaudio` (Linux, BSD): PulseAudio sound server backend. Requires
  `libpulse-dev` (Debian/Ubuntu) or `pulseaudio-libs-devel` (Fedora).

## Known issues
- If the device doesn't support the required sample rate, aliasing may occur.

## How to get it
It is available on [crates.io](https://crates.io/crates/raplay)

## Links
- **Author:** [BonnyAD9](https://github.com/BonnyAD9)
- **GitHub repository:** [BonnyAD/raplay](https://github.com/BonnyAD9/raplay)
- **Package:** [crates.io](https://crates.io/crates/raplay)
- **Documentation:** [docs.rs](https://docs.rs/raplay/latest/raplay/)
- **My Website:** [bonnyad9.github.io](https://bonnyad9.github.io/)
