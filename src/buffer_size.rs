use std::time::Duration;

use cpal::SupportedBufferSize;

#[derive(Copy, Clone, Debug, Default)]
pub enum BufferSize {
    #[default]
    Auto,
    Fixed(u32),
    ByDuration(Duration),
}

impl BufferSize {
    pub fn to_cpal(
        &self,
        limits: &SupportedBufferSize,
        sample_rate: u32,
    ) -> cpal::BufferSize {
        if let SupportedBufferSize::Range { min, max } = limits {
            match self {
                BufferSize::Auto => cpal::BufferSize::Default,
                BufferSize::Fixed(n) => {
                    cpal::BufferSize::Fixed(*n.max(min).min(max))
                }
                BufferSize::ByDuration(d) => {
                    let n = (d.as_secs_f32() * sample_rate as f32) as u32;
                    cpal::BufferSize::Fixed(n.max(*min).min(*max))
                }
            }
        } else {
            cpal::BufferSize::Default
        }
    }
}
