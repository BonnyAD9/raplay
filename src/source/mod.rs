mod symph;

pub trait Source: Iterator<Item = f32> {
    // the sample rate
    fn sample_rate(&self) -> u32;

    // returns the number of samples remaining in the frame
    fn frame_length(&self) -> u32;

    // returns the number of channels
    fn channels(&self) -> u32;
}
