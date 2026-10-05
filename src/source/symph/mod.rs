mod err;
mod options;
mod planes_transmute;

pub use self::{err::*, options::*};

use std::{fmt::Debug, fs::File, path::Path, time::Duration};

use cpal::SampleFormat;
use symphonia::{
    core::{
        audio::GenericAudioBufferRef,
        codecs::{CodecParameters, audio::AudioDecoder},
        formats::{FormatReader, SeekMode, SeekTo, TrackType},
        io::{MediaSourceStream, MediaSourceStreamOptions},
        units::{self, Time, TimeBase, Timestamp},
    },
    default::{get_codecs, get_probe},
};

use crate::{
    callback::Callback,
    converters::{Convert, UniSample},
    err as cerr, operate_samples,
    sample_buffer::SampleBufferMut,
    source::symph::planes_transmute::PlanesTransmute,
};

use super::{DeviceConfig, Source, VolumeIterator};

pub use symphonia::core::io::MediaSource;

/// Source that decodes audio using symphonia decoder
pub struct Symph {
    /// Output sample format to request.
    sample_format: Option<SampleFormat>,
    /// Converter for channels, rate, types and volume.
    convert: Convert,
    /// The probe for the audio
    probed: Box<dyn FormatReader>,
    /// The decoder for the audio
    decoder: Box<dyn AudioDecoder>,
    /// The track of the file that is played
    track_id: u32,
    /// Time base of the track.
    time_base: Option<TimeBase>,
    /// Total duration of the track.
    duration: Option<units::Duration>,
    /// Index into the buffer, where to start reading next samples
    buffer_start: Option<usize>,
    /// The timestamp of the last frame
    last_ts: Timestamp,
    /// Error callback for recoverable errors.
    err_callback: Callback<cerr::Error>,
}

impl Symph {
    /// Tries to create a new `Symph`
    ///
    /// # Errors
    /// - the format of the source cannot be determined
    /// - no default track is found
    /// - no decoder was found for the codec, insufficient codec parameters
    pub fn try_new<T: MediaSource + 'static>(
        source: T,
        opt: &Options,
    ) -> cerr::Result<Symph> {
        let stream = MediaSourceStream::new(
            Box::new(source),
            MediaSourceStreamOptions::default(),
        );

        let pres = get_probe()
            .probe(
                &Default::default(),
                stream,
                opt.format.clone(),
                Default::default(),
            )
            .map_err(Error::SymphInner)?;

        // TODO: select other track if the default is unavailable
        let track = pres
            .default_track(TrackType::Audio)
            .ok_or(Error::CantSelectTrack)?;
        let track_id = track.id;
        let time_base = track.time_base;
        let duration = track
            .duration
            .or(track.num_frames.map(units::Duration::new));
        let Some(CodecParameters::Audio(params)) = &track.codec_params else {
            return Err(Error::CantSelectTrack.into());
        };

        let decoder = get_codecs()
            .make_audio_decoder(params, &opt.decoder)
            .map_err(Error::SymphInner)?;

        Ok(Symph {
            sample_format: opt.sample_format,
            convert: Convert::default(),
            probed: pres,
            decoder,
            track_id,
            time_base,
            duration,
            buffer_start: None,
            last_ts: Timestamp::ZERO,
            err_callback: Callback::default(),
        })
    }

    /// Create new symphonia source by reading the given file.
    ///
    /// # Errors
    /// - Io error on opening the file.
    /// - The format of the source cannot be determined.
    /// - No default track is found.
    /// - No decoder was found for the codec, insufficient codec parameters.
    pub fn open(p: impl AsRef<Path>, opts: &Options) -> cerr::Result<Self> {
        Self::try_new(
            File::open(p).map_err(|e| Error::SymphInner(e.into()))?,
            opts,
        )
    }
}

impl Source for Symph {
    fn set_err_callback(&mut self, err_callback: &Callback<cerr::Error>) {
        self.err_callback = err_callback.clone();
    }

    fn init(&mut self, info: &DeviceConfig) -> anyhow::Result<()> {
        self.convert.set_dst_channels(info.channel_count as usize);
        self.convert.set_dst_rate(info.sample_rate);
        Ok(())
    }

    fn read(
        &mut self,
        buffer: &mut SampleBufferMut,
    ) -> (usize, anyhow::Result<()>) {
        operate_samples!(buffer, b, {
            let (l, e) = self.decode(b);
            (l, e.map_err(|e| cerr::Error::Symph(e).into()))
        })
    }

    fn preferred_config(&mut self) -> Option<DeviceConfig> {
        let mut dec = self.decoder.last_decoded();
        let mut spec = dec.spec();

        if spec.rate() == 0 && dec.frames() == 0 {
            self.decode_packet().ok()?;
            self.buffer_start = Some(0);
            dec = self.decoder.last_decoded();
            spec = dec.spec();
        }

        Some(DeviceConfig {
            channel_count: spec.channels().count() as u32,
            sample_rate: spec.rate(),
            sample_format: self.sample_format.unwrap_or(match dec {
                GenericAudioBufferRef::U8(_) => SampleFormat::U8,
                GenericAudioBufferRef::U16(_) => SampleFormat::U16,
                GenericAudioBufferRef::U24(_) => SampleFormat::I24,
                GenericAudioBufferRef::U32(_) => SampleFormat::U32,
                GenericAudioBufferRef::S8(_) => SampleFormat::I8,
                GenericAudioBufferRef::S16(_) => SampleFormat::I16,
                GenericAudioBufferRef::S24(_) => SampleFormat::I24,
                GenericAudioBufferRef::S32(_) => SampleFormat::I32,
                GenericAudioBufferRef::F32(_) => SampleFormat::F32,
                GenericAudioBufferRef::F64(_) => SampleFormat::F32,
            }),
        })
    }

    fn volume(&mut self, volume: VolumeIterator) -> bool {
        self.convert.set_volume(volume);
        true
    }

    fn seek(&mut self, time: Duration) -> anyhow::Result<crate::Timestamp> {
        let time = duration_to_time(time).ok_or(Error::TooLargeDuration)?;

        let seek_to = SeekTo::Time {
            time,
            track_id: Some(self.track_id),
        };

        let pos = self.probed.seek(SeekMode::Coarse, seek_to)?;

        self.buffer_start = None;
        self.last_ts = pos.actual_ts;
        self.get_time()
            .ok_or(cerr::Error::CannotDetermineTimestamp.into())
    }

    fn get_time(&self) -> Option<crate::Timestamp> {
        let tb = self.time_base?;

        let cur = tb.calc_time_saturating(self.last_ts);

        let total = if let Some(f) = self.duration {
            tb.calc_time_saturating(Timestamp::new(
                0i64.saturating_add_unsigned(f.get()),
            ))
        } else {
            cur
        };

        let (cs, cn) = cur.parts();
        let (ts, tn) = total.parts();

        Some(crate::Timestamp::new(
            Duration::new(0u64.saturating_add_signed(cs), cn),
            Duration::new(0u64.saturating_add_signed(ts), tn),
        ))
    }
}

impl Symph {
    /// Continues decoding the audio
    fn decode<T: UniSample + 'static>(
        &mut self,
        mut buffer: &mut [T],
    ) -> (usize, Result<(), Error>)
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

        while !buffer.is_empty() {
            match self.decode_packet() {
                Ok(true) => {}
                Ok(false) => return (readed, Ok(())),
                Err(e) => return (readed, Err(e)),
            }

            let i = self.read_buffer(&mut buffer, 0);
            buffer = &mut buffer[i..];
            readed += i;
        }

        (readed, Ok(()))
    }

    /// Decodes the next packet
    fn decode_packet(&mut self) -> Result<bool, Error> {
        loop {
            let packet = loop {
                match self.probed.next_packet() {
                    Ok(Some(p)) => {
                        if p.track_id != self.track_id {
                            continue;
                        }
                        self.last_ts = p.pts;
                        break p;
                    }
                    Ok(None) => return Ok(false),
                    Err(symphonia::core::errors::Error::ResetRequired) => {
                        self.decoder.reset()
                    }
                    Err(e) => return Err(e.into()),
                }
            };

            break match self.decoder.decode(&packet) {
                Ok(d) => {
                    let spec = d.spec();
                    self.convert.set_src_channels(spec.channels().count());
                    self.convert.set_src_rate(spec.rate());
                    Ok(true)
                }
                // Try to recover from recoverable errors.
                Err(symphonia::core::errors::Error::ResetRequired) => continue,
                Err(
                    e @ (symphonia::core::errors::Error::DecodeError(_)
                    | symphonia::core::errors::Error::IoError(_)),
                ) => {
                    _ = self
                        .err_callback
                        .invoke(Error::SymphRecoverable(e).into());
                    continue;
                }
                Err(e) => Err(e.into()),
            };
        }
    }

    /// reads from the decoders buffer into the given buffer, returns number
    /// of written samples
    fn read_buffer<T: UniSample + 'static>(
        &mut self,
        buffer: &mut &mut [T],
        start: usize,
    ) -> usize
    where
        T::Float: From<f32>,
    {
        if buffer.is_empty() {
            return 0;
        }

        let samples = self.decoder.last_decoded();

        macro_rules! arm {
            ($src:ident) => {{
                let (rd, wrt) = self.convert.from_planes($src, start, *buffer);

                self.buffer_start = if $src[0].len() == rd + start {
                    None
                } else {
                    Some(rd + start)
                };
                wrt
            }};
        }

        match samples {
            GenericAudioBufferRef::U8(src) => arm!(src),
            GenericAudioBufferRef::U16(src) => arm!(src),
            GenericAudioBufferRef::U24(src) => {
                let src = PlanesTransmute(src);
                let src = &src;
                arm!(src)
            }
            GenericAudioBufferRef::U32(src) => arm!(src),
            GenericAudioBufferRef::S8(src) => arm!(src),
            GenericAudioBufferRef::S16(src) => arm!(src),
            GenericAudioBufferRef::S24(src) => {
                let src = PlanesTransmute(src);
                let src = &src;
                arm!(src)
            }
            GenericAudioBufferRef::S32(src) => arm!(src),
            GenericAudioBufferRef::F32(src) => arm!(src),
            GenericAudioBufferRef::F64(src) => arm!(src),
        }
    }
}

impl Debug for Symph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Symph")
            .field("convert", &self.convert)
            .field("probed", &"Box<dyn FormatReader>")
            .field("decoder", &"Box<dyn AudioDecoder>")
            .field("track_id", &self.track_id)
            .field("time_base", &self.time_base)
            .field("duration", &self.duration)
            .field("buffer_start", &self.buffer_start)
            .field("last_ts", &self.last_ts)
            .field("err_callback", &self.err_callback)
            .finish()
    }
}

fn duration_to_time(dur: Duration) -> Option<Time> {
    Time::try_new(dur.as_secs().try_into().ok()?, dur.subsec_nanos())
}
