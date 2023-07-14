# raplay
Library for playing audio.

The library is very new and not much tested.

## Features
- Play(Resume)/Pause
- Callback when audio ends
- Callback for errors

## Supported formats
All the decoding is done by
[symphonia](https://github.com/pdeljanov/Symphonia/tree/master), so the
supported formats are the same as symphonia.

## Examples

### Play a sine wave
```rust
use raplay::{Sink, source::SineSource};

let sink = Sink::default_out(); // get the default output
let src = SineSource::new(1000.); // create 1000Hz sine source
sink.load(src, true)?; // play the sine wave
```

### Play a mp3 file
```rust
use std::fs::File;
use raplay::{Sink, source::Symph};

let sink = Sink::default_out(); // get the default output
let file = File::open("music.mp3")?; // open the mp3 file
let src = Symph::try_new(file)?; // create a symphonia decoder source
sink.load(src, true)?; // play the mp3 file
```

## Know issues
- The sound is not clear when playing high sample rates relative to what
is set by the device (192000Hz/41000Hz)
    - this has now been fixed with a workaround and should not be problem
      on most devices
