pub mod converters;
pub mod sample_buffer;
pub mod sink;
pub mod source;

#[cfg(test)]
mod tests {
    use std::{fs::File, io::stdin};

    use crate::{sink::{Sink, ErrCallbackInfo}, source::symph::Symph};
    use eyre::Result;

    #[test]
    fn play_audio() -> Result<()> {
        let sink = Sink::default_out()?;
        let src = Symph::try_new(File::open(
            "/home/kubas/Music/4tet - 1st - 02 How Deep Is Your Love.mp3",
        )?)?;
        sink.on_callback(Some(|_| println!("callback")))?;
        sink.on_err_callback(Some(|e: ErrCallbackInfo| println!("{}", e.err)))?;
        sink.load(src, true)?;
        //thread::sleep(Duration::from_secs(5));
        loop {
            let mut s = String::new();
            _ = stdin().read_line(&mut s);
            sink.play(!sink.is_playing()?)?;
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
