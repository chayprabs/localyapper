//! Start and stop sound cues (setting `sound_cues`, default off).
//!
//! Two short PCM clips are embedded in the binary and played through a
//! `cpal` output stream on a short-lived thread, so the controller never
//! waits for audio output.

use crate::core::audio::wav;
use crate::core::session::Cue;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const START_WAV: &[u8] = include_bytes!("cues_pcm/start.wav");
const STOP_WAV: &[u8] = include_bytes!("cues_pcm/stop.wav");

/// Decoded mono samples and their rate.
pub fn clip(cue: Cue) -> Result<(Vec<f32>, u32), String> {
    let data = wav::parse(match cue {
        Cue::Start => START_WAV,
        Cue::Stop => STOP_WAV,
    })?;
    Ok((data.to_mono(), data.sample_rate))
}

/// Play `cue` on the default output device without blocking.
pub fn play(cue: Cue) {
    let spawned = std::thread::Builder::new()
        .name("sound-cue".into())
        .spawn(move || {
            if let Err(e) = play_blocking(cue) {
                log::debug!("cue: {e}");
            }
        });
    if let Err(e) = spawned {
        log::debug!("cue: {e}");
    }
}

fn play_blocking(cue: Cue) -> Result<(), String> {
    let (samples, rate) = clip(cue)?;
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let out_rate = supported.sample_rate().0;
    let channels = usize::from(supported.channels());
    let samples = if out_rate == rate {
        samples
    } else {
        sherpa_onnx::LinearResampler::create(rate as i32, out_rate as i32)
            .ok_or("no resampler")?
            .resample(&samples, true)
    };
    let duration = Duration::from_millis(samples.len() as u64 * 1000 / u64::from(out_rate.max(1)));
    let samples = Arc::new(samples);
    let pos = Arc::new(AtomicUsize::new(0));
    let config: cpal::StreamConfig = supported.clone().into();
    let err = |e: cpal::StreamError| log::debug!("cue stream: {e}");
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let (s, p) = (samples.clone(), pos.clone());
            device.build_output_stream(
                &config,
                move |out: &mut [f32], _| fill(out, channels, &s, &p, |v| v),
                err,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let (s, p) = (samples.clone(), pos.clone());
            device.build_output_stream(
                &config,
                move |out: &mut [i16], _| fill(out, channels, &s, &p, |v| (v * 32767.0) as i16),
                err,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let (s, p) = (samples.clone(), pos.clone());
            device.build_output_stream(
                &config,
                move |out: &mut [u16], _| {
                    fill(out, channels, &s, &p, |v| {
                        ((v * 32767.0) as i32 + 32768) as u16
                    })
                },
                err,
                None,
            )
        }
        other => return Err(format!("unsupported output format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    std::thread::sleep(duration + Duration::from_millis(60));
    Ok(())
}

/// Copy the next frames of `src` into interleaved `out`, then silence.
fn fill<T: Copy>(
    out: &mut [T],
    channels: usize,
    src: &[f32],
    pos: &AtomicUsize,
    conv: impl Fn(f32) -> T,
) {
    let mut p = pos.load(Ordering::Relaxed);
    for frame in out.chunks_mut(channels.max(1)) {
        let v = src.get(p).copied().unwrap_or(0.0);
        p += 1;
        for s in frame {
            *s = conv(v);
        }
    }
    pos.store(p, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_are_short_quiet_and_distinct() {
        let (start, rate) = clip(Cue::Start).expect("start clip");
        let (stop, _) = clip(Cue::Stop).expect("stop clip");
        assert_eq!(rate, 48_000);
        assert!(start.len() < rate as usize / 5, "under 200 ms");
        assert!(start.iter().all(|v| v.abs() <= 0.25));
        assert_ne!(start, stop);
    }

    #[test]
    fn fill_writes_every_channel_then_silence() {
        let pos = AtomicUsize::new(0);
        let mut out = [9.0f32; 6];
        fill(&mut out, 2, &[0.5, 0.25], &pos, |v| v);
        assert_eq!(out, [0.5, 0.5, 0.25, 0.25, 0.0, 0.0]);
        assert_eq!(pos.load(Ordering::Relaxed), 3);
    }
}
