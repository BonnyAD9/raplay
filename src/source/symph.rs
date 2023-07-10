use eyre::{Report, Result};
use symphonia::{
    core::{
        codecs::Decoder,
        io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions},
    },
    default::{get_codecs, get_probe},
};

use super::Source;

pub struct Symph {
    sample_rate: u32,
    channels: u32,
    decoder: Box<dyn Decoder>,
}

impl Symph {
    pub fn try_new<T: MediaSource + 'static>(source: T) -> Result<Symph> {
        let stream = MediaSourceStream::new(
            Box::new(source),
            MediaSourceStreamOptions::default(),
        );

        let pres = get_probe().format(
            &Default::default(),
            stream,
            &Default::default(),
            &Default::default(),
        )?;

        let track = pres
            .format
            .default_track()
            .ok_or(Report::msg("Cannot get default track"))?;

        let decoder =
            get_codecs().make(&track.codec_params, &Default::default())?;

        Ok(Symph {
            sample_rate: track
                .codec_params
                .sample_rate
                .ok_or(Report::msg("Cannot determine sample rate"))?,

            channels: track
                .codec_params
                .channels
                .ok_or(Report::msg("Cannot determine channels"))?
                .count() as u32,

            decoder,
        })
    }
}

impl Source for Symph {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn frame_length(&self) -> u32 {
        todo!()
    }

    fn channels(&self) -> u32 {
        self.channels
    }
}

impl Iterator for Symph {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
