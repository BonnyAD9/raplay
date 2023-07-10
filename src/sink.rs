use cpal::traits::{DeviceTrait, HostTrait};
use eyre::{Report, Result};

pub struct Sink {}

impl Sink {
    pub fn default_out() -> Result<()> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(Report::msg("No available output device"))?;
        let config = device
            .supported_output_configs()?
            .next()
            .ok_or(Report::msg("No supported device config"))?
            .with_max_sample_rate();
        let sample_format = config.sample_format();
        let config = config.into();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                Sink::write_silence::<f32>,
                |_| {},
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                Sink::write_silence::<i16>,
                |_| {},
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                Sink::write_silence::<u16>,
                |_| {},
                None,
            ),
            sample_format => {
                panic!("Unsupported sample format '{sample_format}'")
            }
        }
        .unwrap();

        Ok(())
    }

    fn write_silence<T: cpal::Sample>(
        data: &mut [T],
        info: &cpal::OutputCallbackInfo,
    ) {
        for sample in data.iter_mut() {
            *sample = T::EQUILIBRIUM;
        }
    }
}
