pub mod converters;
pub mod err;
pub mod sample_buffer;
pub mod sink;
pub mod source;

pub use err::Error;
pub use sink::Sink;

#[cfg(test)]
mod tests {
    use std::{fs::File, io::stdin, time::Duration};

    use crate::{err::Error, source::Symph, Sink};

    use anyhow::Result;

    #[test]
    fn play_audio() -> Result<()> {
        let mut sink = Sink::default();
        let src = Symph::try_new(File::open(
            "/home/kubas/Music/AJR - Neotheater - 01 Next Up Forever.flac",
        )?)?;
        sink.on_callback(Some(|_| println!("callback")))?;
        sink.on_err_callback(Some(|e: Error| println!("{}", e)))?;
        sink.volume(0.2)?;
        sink.load(src, true)?;
        sink.seek_to(Duration::from_secs_f32(60.0))?;
        //thread::sleep(Duration::from_secs(5));
        loop {
            let mut s = String::new();
            _ = stdin().read_line(&mut s);
            sink.play(!sink.is_playing()?)?;
            let ts = sink.get_timestamp()?;
            println!("{:?}/{:?}", ts.0, ts.1);
        }
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
