mod buffer_size;
pub mod converters;
pub mod err;
mod mixer;
pub mod sample_buffer;
mod shared;
pub mod sink;
pub mod source;
mod timestamp;

///! Library for playing audio
pub use self::{
    buffer_size::BufferSize, err::Error, sink::Sink, timestamp::Timestamp,
};

#[cfg(test)]
mod tests {
    /*use std::{fs::File, io::stdin, time::Duration};

    use crate::{err::Error, source::Symph, Sink};

    use anyhow::Result;

    #[test]
    fn play_audio() -> Result<()> {
        let mut sink = Sink::default();
        let src = Symph::try_new(File::open(
            "/mnt/x/Music/AJR - Neotheater - 01 Next Up Forever.flac",
        )?, &Default::default())?;
        sink.on_callback(Some(|c| println!("callback: {c:?}")))?;
        sink.on_err_callback(Some(|e: Error| println!("{}", e)))?;
        sink.volume(0.2)?;
        sink.set_buffer_size(Some(1024));
        sink.load(src, true)?;
        sink.set_fade_len(Duration::from_millis(200))?;
        //thread::sleep(Duration::from_secs(5));
        loop {
            let mut s = String::new();
            _ = stdin().read_line(&mut s);
            sink.play(!sink.is_playing()?)?;
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
