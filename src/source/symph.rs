use cpal::{SampleFormat, I24, U24};
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
    converters::{do_channels_rate, interleave, UniSample},
    operate_samples,
    sample_buffer::SampleBufferMut,
};

use super::{DeviceConfig, Source, VolumeIterator};

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
    volume: VolumeIterator,
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
            volume: VolumeIterator::constant(1.),
        })
    }
}

impl Source for Symph {
    fn init(&mut self, info: &DeviceConfig) -> Result<()> {
        self.target_sample_rate = info.sample_rate;
        self.target_channels = info.channel_count;
        Ok(())
    }

    fn read(&mut self, buffer: &mut SampleBufferMut) -> (usize, Result<()>) {
        operate_samples!(buffer, b, self.decode(*b))
    }

    fn preffered_config(&mut self) -> Option<DeviceConfig> {
        let mut dec = self.decoder.last_decoded();
        let mut spec = dec.spec();

        if spec.rate == 0 && dec.frames() == 0 {
            self.decode_packet().ok()?;
            self.buffer_start = Some(0);
            dec = self.decoder.last_decoded();
            spec = dec.spec();
        }

        Some(DeviceConfig {
            channel_count: spec.channels.count() as u32,
            sample_rate: spec.rate,
            sample_format: match dec {
                AudioBufferRef::U8(_) => SampleFormat::U8,
                AudioBufferRef::U16(_) => SampleFormat::U16,
                AudioBufferRef::U24(_) => SampleFormat::F32,
                AudioBufferRef::U32(_) => SampleFormat::U32,
                AudioBufferRef::S8(_) => SampleFormat::I8,
                AudioBufferRef::S16(_) => SampleFormat::I16,
                AudioBufferRef::S24(_) => SampleFormat::F32,
                AudioBufferRef::S32(_) => SampleFormat::I32,
                AudioBufferRef::F32(_) => SampleFormat::F32,
                AudioBufferRef::F64(_) => SampleFormat::F32,
            },
        })
    }

    fn volume(&mut self, volume: VolumeIterator) -> bool {
        self.volume = volume;
        true
    }
}

impl Symph {
    fn decode<T: UniSample>(
        &mut self,
        mut buffer: &mut [T],
    ) -> (usize, Result<()>)
    where
        T::Float: From<f32>,
    {
        // TODO: no temp buffer
        let mut readed = 0;

        if let Some(index) = self.buffer_start {
            // self.buffer is Some because self.buffer_start is Some
            let i = self.read_buffer(&mut buffer, index);
            buffer = &mut buffer[i..];
            readed += i;
        }

        while buffer.len() > 0 {
            match self.decode_packet() {
                Ok(_) => {},
                Err(e) => return (readed, Err(e))
            }

            // self.buffer is always Some
            let i = self.read_buffer(&mut buffer, 0);
            buffer = &mut buffer[i..];
            readed += i;
        }

        (readed, Ok(()))
    }

    fn decode_packet(&mut self) -> Result<()> {
        let packet = loop {
            match self.probed.format.next_packet() {
                Ok(p) => {
                    if p.track_id() != self.track_id {
                        continue;
                    }
                    break p;
                }
                // TODO: check for ResetRequired
                Err(e) => return Err(Report::new(e)),
            }
        };

        match self.decoder.decode(&packet) {
            Ok(d) => {
                self.source_sample_rate = d.spec().rate;
                self.source_channels = d.spec().channels.count() as u32;
                Ok(())
            }
            Err(e) => Err(Report::new(e)),
        }
    }

    /// self.buffer must be some
    fn read_buffer<T: UniSample>(
        &mut self,
        buffer: &mut &mut [T],
        start: usize,
    ) -> usize
    where
        T::Float: From<f32>,
    {
        let samples = self.decoder.last_decoded();
        let mut i = 0;

        macro_rules! arm {
            ($mnam:ident, $map:expr, $src:ident) => {{
                let mut len = 0;
                let mut last_index = 0;
                for s in do_channels_rate(
                    interleave($src.planes().planes().iter().map(|i| {
                        let slice =
                            &i[start / self.source_channels as usize..];
                        len += slice.len();
                        slice.iter()
                    }))
                    .map(|$mnam| {
                        last_index += 1;
                        $map
                    }),
                    self.source_channels,
                    self.target_channels,
                    self.source_sample_rate,
                    self.target_sample_rate,
                ) {
                    buffer[i] = T::from_sample(s)
                        .mul_amp(self.volume.next_vol().into());
                    i += 1;
                    if i == buffer.len() {
                        break;
                    }
                }

                self.buffer_start = if last_index == len {
                    None
                } else {
                    Some(last_index)
                }
            }};
        }

        match samples {
            AudioBufferRef::U8(src) => arm!(s, *s, src),
            AudioBufferRef::U16(src) => arm!(s, *s, src),
            AudioBufferRef::U24(src) => {
                arm!(s, U24::new(s.clamped().0 as i32).unwrap(), src)
            }
            AudioBufferRef::U32(src) => arm!(s, *s, src),
            AudioBufferRef::S8(src) => arm!(s, *s, src),
            AudioBufferRef::S16(src) => arm!(s, *s, src),
            AudioBufferRef::S24(src) => {
                arm!(s, I24::new(s.clamped().0).unwrap(), src)
            }
            AudioBufferRef::S32(src) => arm!(s, *s, src),
            AudioBufferRef::F32(src) => arm!(s, *s, src),
            AudioBufferRef::F64(src) => arm!(s, *s, src),
            //AudioBufferRef::S32(src) => arm!(s, *s, src),
        }

        i
    }
}
