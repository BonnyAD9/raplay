use cpal::SampleFormat;
pub use symphonia::core::codecs::audio::AudioDecoderOptions;
pub use symphonia::core::formats::FormatOptions;

#[non_exhaustive]
#[derive(Debug, Default)]
pub struct Options {
    pub format: FormatOptions,
    pub decoder: AudioDecoderOptions,
    /// Sample format that this source will request as output. If none, it will
    /// be chosen based on the source sample format.
    ///
    /// If you plan to use volume or resampling, you may consider changing this
    /// to SampleFormat::F32 because the samples will have to be converted to
    /// f32 for processing. This will prevent the conversion to another type
    /// and potentially decrease resource usage.
    pub sample_format: Option<SampleFormat>,
}
