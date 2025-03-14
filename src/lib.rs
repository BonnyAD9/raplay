//! # raplay
//! Library for playing audio
//!
//! ## Examples
//! ### Play a sine wave
//! ```rust,ignore
//! use raplay::{Sink, source::SineSource}
//!
//! let sink = Sink::default(); // Get the default output
//! let src = SineSource::new(1000.); // Create 1000Hz sine source
//! sink.load(src, true)?; // Play the sine wave
//! ```
//!
//! ### Play a mp3 file
//! ```rust,ignore
//! use std::fs::File;
//! use raplay::{Sink, source::Symph}
//!
//! let sink = Sink::default(); // Get the default output
//! let file = File::open("music.mp3")?; // Open the mp3 file
//! let src = Symph::try_new(file, &Default::default())?; // Create a symphonia
//!                                                       // decoder source
//! sink.load(src, true); // Play the mp3 file
//! ```
//!
//! ## Known issues
//! - If the output device doesn't support the required sample rate, aliasing
//!   may occur.

/// Useful conversions on samples.
pub mod converters;
/// Useful reexports.
pub mod reexp;
pub mod sample_buffer;
pub mod sink;
/// Audio sources that can be played in [`Sink`].
pub mod source;

mod buffer_size;
mod callback;
mod callback_info;
mod controls;
mod err;
mod mixer;
mod shared_data;
mod timestamp;

pub(crate) use self::{controls::*, shared_data::*};

pub use self::{
    buffer_size::*, callback::*, callback_info::*, err::*, sink::Sink,
    source::Source, timestamp::*,
};

#[cfg(test)]
mod tests {
    /*use std::{fs::File, io::stdin, time::Duration};

    use crate::{err::Error, source::Symph, BufferSize, Sink};

    use anyhow::Result;
    use cpal::traits::DeviceTrait;

    #[test]
    fn play_audio() -> Result<()> {
        let home_path = include_str!("../tmp/home").to_owned();

        let mut sink = Sink::default();
        let src = Symph::try_new(
            File::open(
                home_path + "/music/4tet - 4th -03 Air.mp3",
                //HOME_PATH + "/music/AJR - Neotheater - 01 Next Up Forever.flac",
            )?,
            &Default::default(),
        )?;
        sink.on_callback(Box::new(|c| println!("callback: {c:?}")))?;
        sink.on_err_callback(Box::new(|e: Error| println!("{}", e)))?;
        sink.volume(0.3)?;
        /*for i in Sink::list_devices()? {
            println!("{}", i.name()?);
        }*/
        sink.load(Box::new(src), true)?;
        sink.set_fade_len(Duration::from_millis(200))?;
        //thread::sleep(Duration::from_secs(5));
        loop {
            let mut s = String::new();
            _ = stdin().read_line(&mut s);
            //sink.play(!sink.is_playing()?)?;
            let ts = sink.get_timestamp()?;
            println!("{:?}/{:?}", ts.current, ts.total);
        }
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
