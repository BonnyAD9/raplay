use cpal::{I24, U24};
use eyre::{Report, Result};
use symphonia::{
    core::{
        audio::AudioBufferRef,
        codecs::Decoder,
        io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions},
        probe::ProbeResult,
        sample::Sample,
    },
    default::{get_codecs, get_probe},
};

use crate::{
    converters::{do_interleave_channels_rate, UniSample},
    operate_samples,
    sample_buffer::SampleBufferMut,
};

use super::Source;

/// Source that decodes audio using symphonia decoder
pub struct Symph {
    target_sample_rate: u32,
    target_channels: u32,
    source_sample_rate: u32,
    source_channels: u32,
    probed: ProbeResult,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    buffer_start: Option<usize>,
}

impl Symph {
    /// Tries to create a new `Symph`
    ///
    /// # Errors
    /// - the format of the source cannot be determined
    /// - no default track is found
    /// - no decoder was found for the codec, insufficient codec parameters
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

        // TODO: select other track if the default is unavailable
        let track = pres
            .format
            .default_track()
            .ok_or(Report::msg("Cannot get default track"))?;
        let track_id = track.id;

        let decoder =
            get_codecs().make(&track.codec_params, &Default::default())?;

        Ok(Symph {
            target_sample_rate: 0,
            target_channels: 0,
            source_channels: 0,
            source_sample_rate: 0,
            probed: pres,
            decoder,
            track_id,
            buffer_start: None,
        })
    }
}

impl Source for Symph {
    fn init(&mut self, info: &super::DeviceInfo) -> Result<()> {
        self.target_sample_rate = info.sample_rate;
        self.target_channels = info.channel_count;
        Ok(())
    }

    fn read(&mut self, buffer: &mut SampleBufferMut) -> (usize, Result<()>) {
        operate_samples!(buffer, b, self.decode(*b))
    }
}

impl Symph {
    fn decode<T: UniSample>(
        &mut self,
        mut buffer: &mut [T],
    ) -> (usize, Result<()>) {
        // TODO: no temp buffer
        let mut readed = 0;

        if let Some(index) = self.buffer_start {
            // self.buffer is Some because self.buffer_start is Some
            let i = self.read_buffer(&mut buffer, index);
            buffer = &mut buffer[i..];
            readed += i;
        }

        while buffer.len() > 0 {
            let packet = loop {
                match self.probed.format.next_packet() {
                    Ok(p) => {
                        if p.track_id() != self.track_id {
                            continue;
                        }
                        break p;
                    }
                    // TODO: check for ResetRequired
                    Err(e) => return (0, Err(Report::new(e))),
                }
            };

            match self.decoder.decode(&packet) {
                Ok(d) => {
                    self.source_sample_rate = d.spec().rate;
                    self.source_channels = d.spec().channels.count() as u32;
                }
                Err(e) => return (readed, Err(Report::new(e))),
            };

            // self.buffer is always Some
            let i = self.read_buffer(&mut buffer, 0);
            buffer = &mut buffer[i..];
            readed += i;
        }

        (readed, Ok(()))
    }

    /// self.buffer must be some
    fn read_buffer<T: UniSample>(
        &mut self,
        buffer: &mut &mut [T],
        start: usize,
    ) -> usize {
        let samples = self.decoder.last_decoded();
        let mut i = 0;

        macro_rules! arm {
            ($map:expr, $src:ident) => {{
                let mut len = 0;
                for s in do_interleave_channels_rate(
                    $src.planes().planes().iter().map(|i| {
                        let slice =
                            &i[start / self.source_channels as usize..];
                        len += slice.len();
                        slice.iter().map($map)
                    }),
                    self.source_channels,
                    self.target_channels,
                    self.source_sample_rate,
                    self.target_sample_rate,
                ) {
                    buffer[i] = T::from_sample(s);
                    i += 1;
                    if i == buffer.len() {
                        break;
                    }
                }

                self.buffer_start = if i + start == len {
                    None
                } else {
                    Some(i + start)
                }
            }};
        }

        match samples {
            AudioBufferRef::U8(src) => arm!(|s| *s, src),
            AudioBufferRef::U16(src) => arm!(|s| *s, src),
            AudioBufferRef::U24(src) => {
                arm!(|s| U24::new(s.clamped().0 as i32).unwrap(), src)
            }
            AudioBufferRef::U32(src) => arm!(|s| *s, src),
            AudioBufferRef::S8(src) => arm!(|s| *s, src),
            AudioBufferRef::S16(src) => arm!(|s| *s, src),
            AudioBufferRef::S24(src) => {
                arm!(|s| I24::new(s.clamped().0).unwrap(), src)
            }
            AudioBufferRef::S32(src) => arm!(|s| *s, src),
            AudioBufferRef::F32(src) => arm!(|s| *s, src),
            AudioBufferRef::F64(src) => arm!(|s| *s, src),
        }

        i
    }
}
