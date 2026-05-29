pub use symphonia::core::codecs::audio::AudioDecoderOptions;
pub use symphonia::core::formats::FormatOptions;

#[derive(Debug, Default)]
pub struct Options {
    pub format: FormatOptions,
    pub decoder: AudioDecoderOptions,
}
