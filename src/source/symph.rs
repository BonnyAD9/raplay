use eyre::{Report, Result};
use symphonia::{
    core::{
        audio::SampleBuffer,
        codecs::Decoder,
        io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions},
        probe::ProbeResult,
    },
    default::{get_codecs, get_probe},
};

use crate::{
    converters::do_channels_rate, operate_samples,
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
    buffer: Option<SampleBuffer<f32>>, // TODO: generic type to avoid copying of data
    buffer_start: Option<usize>,
}

impl Symph {
    /// Tries to create a new `Symph`
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
            buffer: None,
            buffer_start: None,
        })
    }
}

impl Source for Symph {
    fn init(&mut self, info: &super::DeviceInfo) {
        self.target_sample_rate = info.sample_rate;
        self.target_channels = info.channel_count;
    }

    fn read(&mut self, buffer: &mut SampleBufferMut) -> usize {
        operate_samples!(buffer, b, self.decode(*b))
    }
}

impl Symph {
    fn decode<T: cpal::Sample + cpal::FromSample<f32>>(
        &mut self,
        mut buffer: &mut [T],
    ) -> usize {
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
                if let Ok(p) = self.probed.format.next_packet() {
                    if p.track_id() != self.track_id {
                        continue;
                    }
                    break p;
                }
                // TODO: check for ResetRequired
                return 0;
            };

            // TODO: remove unwrap
            let data = self.decoder.decode(&packet).unwrap();
            self.source_sample_rate = data.spec().rate;
            self.source_channels = data.spec().channels.count() as u32;

            // create new buffer if there is no buffer
            if self.buffer.is_none() {
                self.buffer = Some(SampleBuffer::new(
                    data.capacity() as u64,
                    *data.spec(),
                ));
            }

            // TODO: compensate possible missing samples
            self.buffer.as_mut().unwrap().copy_interleaved_ref(data);

            // self.buffer is always Some
            let i = self.read_buffer(&mut buffer, 0);
            buffer = &mut buffer[i..];
            readed += i;
        }

        readed
    }

    /// self.buffer must be some
    fn read_buffer<T: cpal::Sample + cpal::FromSample<f32>>(
        &mut self,
        buffer: &mut &mut [T],
        start: usize,
    ) -> usize {
        let samples = self.buffer.as_ref().unwrap();
        let mut i = 0;

        for s in do_channels_rate(
            samples.samples()[start..].iter().map(|i| *i),
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

        self.buffer_start = if i + start == self.buffer.as_ref().unwrap().len()
        {
            None
        } else {
            Some(i + start)
        };

        i
    }
}
