//! Minimal RIFF/WAVE reader for tests, the bench harness and the fake
//! audio source. Supports PCM 8/16/24/32-bit integer and 32/64-bit float,
//! any channel count. Audio is never written to disk, so there is no writer.

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct WavData {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved samples in `[-1, 1]`.
    pub samples: Vec<f32>,
}

impl WavData {
    /// Number of frames (samples per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        self.frames() as u64 * 1000 / u64::from(self.sample_rate)
    }

    /// Average the channels into one.
    pub fn to_mono(&self) -> Vec<f32> {
        let ch = usize::from(self.channels.max(1));
        if ch == 1 {
            return self.samples.clone();
        }
        self.samples
            .chunks_exact(ch)
            .map(|f| f.iter().sum::<f32>() / ch as f32)
            .collect()
    }
}

pub fn read_file(path: &Path) -> Result<WavData, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn u16_at(b: &[u8], i: usize) -> Result<u16, String> {
    b.get(i..i + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| "truncated header".to_string())
}

fn u32_at(b: &[u8], i: usize) -> Result<u32, String> {
    b.get(i..i + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| "truncated header".to_string())
}

pub fn parse(b: &[u8]) -> Result<WavData, String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut pos = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= b.len() {
        let id = &b[pos..pos + 4];
        let size = u32_at(b, pos + 4)? as usize;
        let body_start = pos + 8;
        let body_end = body_start.saturating_add(size).min(b.len());
        let body = &b[body_start..body_end];
        if id == b"fmt " {
            let mut format = u16_at(body, 0)?;
            let channels = u16_at(body, 2)?;
            let rate = u32_at(body, 4)?;
            let bits = u16_at(body, 14)?;
            // WAVE_FORMAT_EXTENSIBLE: the real format is the first two
            // bytes of the sub-format GUID.
            if format == 0xFFFE && body.len() >= 26 {
                format = u16_at(body, 24)?;
            }
            fmt = Some((format, channels, rate, bits));
        } else if id == b"data" {
            data = Some(body);
        }
        pos = body_start + size + (size & 1);
    }
    let (format, channels, sample_rate, bits) = fmt.ok_or("missing fmt chunk")?;
    let data = data.ok_or("missing data chunk")?;
    if channels == 0 || sample_rate == 0 {
        return Err("invalid fmt chunk".into());
    }
    let samples: Vec<f32> = match (format, bits) {
        (1, 8) => data
            .iter()
            .map(|&v| (f32::from(v) - 128.0) / 128.0)
            .collect(),
        (1, 16) => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.0)
            .collect(),
        (1, 24) => data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|s| {
                let v = i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8;
                v as f32 / 8_388_608.0
            })
            .collect(),
        (1, 32) => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0)
            .collect(),
        (3, 32) => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|s| f32::from_le_bytes([s[0], s[1], s[2], s[3]]))
            .collect(),
        (3, 64) => data
            .as_chunks::<8>()
            .0
            .iter()
            .map(|s| f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]) as f32)
            .collect(),
        _ => return Err(format!("unsupported WAV format {format} with {bits} bits")),
    };
    Ok(WavData {
        sample_rate,
        channels,
        samples,
    })
}

#[cfg(test)]
pub(crate) fn encode_pcm16(rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_pcm16_stereo() {
        let src = [0.0f32, 0.5, -0.5, 0.25];
        let wav = parse(&encode_pcm16(44_100, 2, &src)).expect("parse");
        assert_eq!(wav.sample_rate, 44_100);
        assert_eq!(wav.channels, 2);
        assert_eq!(wav.frames(), 2);
        for (a, b) in wav.samples.iter().zip(src.iter()) {
            assert!((a - b).abs() < 1e-3);
        }
        let mono = wav.to_mono();
        assert!((mono[0] - 0.25).abs() < 1e-3);
        assert!((mono[1] - (-0.125)).abs() < 1e-3);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"hello").is_err());
        assert!(parse(b"RIFF\0\0\0\0WAVE").is_err());
    }
}
