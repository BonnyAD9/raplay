pub mod converters;
pub mod sample_buffer;
pub mod sink;
pub mod source;

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::{sink::Sink, source::symph::Symph};
    use eyre::Result;

    #[test]
    fn play_audio() -> Result<()> {
        let sink = Sink::default_out()?;
        let src = Symph::try_new(File::open("/home/kubas/Music/4tet - 1st - 02 How Deep Is Your Love.mp3")?)?;
        sink.play(src)?;
        //thread::sleep(Duration::from_secs(5));
        loop {}
    }

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
