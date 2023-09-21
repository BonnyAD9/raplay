# raplay
Library for playing audio.

The library is very new and not much tested.

## Features
- Play(Resume)/Pause
- Callback when audio ends
- Callback for errors
- Volume
- Seeking
- Get audio position and length
- Fade-in/fade-out on play/pause

## Supported formats
All the decoding is done by
[symphonia](https://github.com/pdeljanov/Symphonia/tree/master), so the
supported formats are the same as symphonia.

## Examples

### Play a sine wave
```rust
use raplay::{Sink, source::SineSource};

let sink = Sink::default(); // get the default output
let src = SineSource::new(1000.); // create 1000Hz sine source
sink.load(src, true)?; // play the sine wave
```

### Play a mp3 file
```rust
use std::fs::File;
use raplay::{Sink, source::Symph};

let sink = Sink::default(); // get the default output
let file = File::open("music.mp3")?; // open the mp3 file
let src = Symph::try_new(file, &Default::default())?; // create a symphonia decoder source
sink.load(src, true)?; // play the mp3 file
```

## Known issues
- If the device doesn't support the required sample rate, aliasing may occur

## How to get it
It is available on [crates.io](https://crates.io/crates/raplay)

## Links
- **Author:** [BonnyAD9](https://github.com/BonnyAD9)
- **GitHub repository:** [BonnyAD/raplay](https://github.com/BonnyAD9/raplay)
- **Package:** [crates.io](https://crates.io/crates/raplay)
- **Documentation:** [docs.rs](https://docs.rs/raplay/latest/raplay/)
- **My Website:** [bonnyad9.github.io](https://bonnyad9.github.io/)
