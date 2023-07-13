pub mod converters;
pub mod sample_buffer;
pub mod sink;
pub mod source;

#[cfg(test)]
mod tests {
    use std::{fs::File, thread, time::Duration};

    use crate::{sink::Sink, source::{symph::Symph, sine::SineSource}};
    use eyre::Result;

    use super::*;

    /*#[test]
    fn play_audio() -> Result<()> {
        let sink = Sink::default_out()?;
        let src = Symph::try_new(File::open("/home/kubas/Music/AJR - Neotheater - 01 Next Up Forever.flac")?)?;
        sink.play(src)?;
        thread::sleep(Duration::from_secs(5));
        Ok(())
    }*/

    #[test]
    fn play_sine() -> Result<()> {
        let sink = Sink::default_out()?;
        let src = SineSource::new(100.);
        sink.play(src)?;
        println!("hi");
        loop {}
        Ok(())
    }
}
