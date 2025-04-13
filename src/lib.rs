//! # raplay
//! Library for playing audio
//!
//! ## Examples
//! ### Play a sine wave
//! ```no_run
//! use raplay::{Sink, source::Sine};
//!
//! let mut sink = Sink::default(); // Get the default output
//! let src = Sine::new(1000.); // Create 1000Hz sine source
//! sink.load(Box::new(src), true)?; // Play the sine wave
//! # Ok::<(), raplay::Error>(())
//! ```
//!
//! ### Play a mp3 file
//! ```no_run
//! use std::fs::File;
//! use raplay::{Sink, source::Symph};
//!
//! let mut sink = Sink::default(); // Get the default output
//! let file = File::open("music.mp3").unwrap(); // Open the mp3 file
//! let src = Symph::try_new(file, &Default::default())?; // Create a symphonia
//!                                                       // decoder source
//! sink.load(Box::new(src), true); // Play the mp3 file
//! # Ok::<(), raplay::Error>(())
//! ```
//!
//! ## Known issues
//! - If the output device doesn't support the required sample rate, aliasing
//!   may occur.

/// Useful conversions on samples.
pub mod converters;
/// Useful reexports.
pub mod reexp;
/// Audio sources that can be played in [`Sink`].
pub mod source;

mod buffer_size;
mod callback;
mod callback_info;
mod controls;
mod err;
mod mixer;
mod prefetch_state;
mod sample_buffer;
mod shared_data;
mod sink;
mod timestamp;

pub(crate) use self::{controls::*, shared_data::*};

pub use self::{
    buffer_size::*, callback::*, callback_info::*, err::*, prefetch_state::*,
    sample_buffer::*, sink::*, source::Source, timestamp::*,
};

#[cfg(test)]
mod tests {
    /*use std::{fs::File, io::stdin, path::Path, thread, time::Duration};

    use crate::{BufferSize, Sink, err::Error, source::Symph};

    use anyhow::Result;
    use cpal::traits::DeviceTrait;

    #[test]
    fn play_audio() -> Result<()> {
        let mut sink = Sink::default();
        sink.on_callback(Box::new(|c| eprintln!("callback: {c:?}")))?;
        sink.on_err_callback(Box::new(|e: Error| eprintln!("err: {:?}", e)))?;
        sink.volume(0.1 * 0.1)?;
        sink.prefetch_notify(Duration::from_secs(1))?;

        let src = open_symph("music/4tet - 4th -03 Air.mp3")?;
        let src1 = open_symph(
            "music/Jacob Collier - Djesse Vol. 4/01. 100,000 Voices.flac",
        )?;
        let src2 = open_symph(
            "music/Jacob Collier - Djesse Vol. 4/02. She Put Sunshine.flac",
        )?;
        /*for i in Sink::list_devices()? {
            println!("{}", i.name()?);
        }*/
        sink.load(Box::new(src1), true)?;
        sink.prefetch(Some(Box::new(src2)))?;
        sink.set_fade_len(Duration::from_millis(200))?;
        sink.seek_to(Duration::from_secs(60 * 4 + 40))?;
        //thread::sleep(Duration::MAX);
        loop {
            let mut s = String::new();
            _ = stdin().read_line(&mut s);
            //sink.play(!sink.is_playing()?)?;
            let ts = sink.get_timestamp()?;
            println!("{:?}/{:?}", ts.current, ts.total);
        }
    }

    fn open_symph(p: impl AsRef<Path>) -> Result<Symph> {
        let home_path = include_str!("../tmp/home").to_owned();
        Ok(Symph::try_new(
            File::open(Path::new(&home_path).join(p))?,
            &Default::default(),
        )?)
    }*/

    /*#[test]
    fn play_sine() -> Result<()> {
        let sink = Sink::default_out()?;
        let src = SineSource::new(100.);
        sink.play(src)?;
        println!("hi");
        loop {}
        Ok(())
    }*/
}
