# raplay
Library for playing audio.

The library is very new and not much tested and is messing some important
features (such as pausing the audio).

## Supported formats
All the decoding is done by [symphonia](https://github.com/pdeljanov/Symphonia/tree/master), so the supported formats are the same as symphonia.

## Examples

### Play a sine wave
```rust
use raplay::{Sink, source::SineSource};

let sink = Sink::default_out(); // get the default output
let src = SineSource::new(1000.); // create 1000Hz sine source
sink.play(src)?; // play the sine wave
```

### Play a mp3 file
```rust
use std::fs::File;
use raplay::{Sink, source::Symph};

let sink = Sink::default_out(); // get the default output
let file = File::open("music.mp3")?; // open the mp3 file
let src = Symph::try_new(file)?; // create a symphonia decoder source
sink.play(src)?; // play the mp3 file
```
