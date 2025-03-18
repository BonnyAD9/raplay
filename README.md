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
