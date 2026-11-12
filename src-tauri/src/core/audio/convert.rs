//! Sample format conversion and downmixing. Used inside the audio
//! callback, so nothing here allocates.

/// A device sample that converts to `f32` in `[-1, 1]`.
pub trait ToF32: Copy {
    fn to_f32(self) -> f32;
}

impl ToF32 for f32 {
    #[inline]
    fn to_f32(self) -> f32 {
        self
    }
}

impl ToF32 for i8 {
    #[inline]
    fn to_f32(self) -> f32 {
        f32::from(self) / 128.0
    }
}

impl ToF32 for i16 {
    #[inline]
    fn to_f32(self) -> f32 {
        f32::from(self) / 32_768.0
    }
}

impl ToF32 for i32 {
    #[inline]
    fn to_f32(self) -> f32 {
        self as f32 / 2_147_483_648.0
    }
}

impl ToF32 for u8 {
    #[inline]
    fn to_f32(self) -> f32 {
        (f32::from(self) - 128.0) / 128.0
    }
}

impl ToF32 for u16 {
    #[inline]
    fn to_f32(self) -> f32 {
        (f32::from(self) - 32_768.0) / 32_768.0
    }
}

/// Downmix interleaved `input` with `channels` channels into mono `out`.
/// Converts as many whole frames as fit in `out` and returns how many
/// mono samples were written. A trailing partial frame is ignored.
#[inline]
pub fn downmix_into<T: ToF32>(input: &[T], channels: usize, out: &mut [f32]) -> usize {
    let channels = channels.max(1);
    let frames = (input.len() / channels).min(out.len());
    if channels == 1 {
        for (o, s) in out.iter_mut().zip(input.iter()).take(frames) {
            *o = s.to_f32();
        }
        return frames;
    }
    let scale = 1.0 / channels as f32;
    for (o, frame) in out
        .iter_mut()
        .zip(input.chunks_exact(channels))
        .take(frames)
    {
        let mut acc = 0.0f32;
        for s in frame {
            acc += s.to_f32();
        }
        *o = acc * scale;
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_formats_map_to_unit_range() {
        assert_eq!(i16::MIN.to_f32(), -1.0);
        assert_eq!(0i16.to_f32(), 0.0);
        assert!((i16::MAX.to_f32() - 1.0).abs() < 1e-4);
        assert_eq!(0u16.to_f32(), -1.0);
        assert_eq!(32_768u16.to_f32(), 0.0);
        assert_eq!(128u8.to_f32(), 0.0);
        assert_eq!(0u8.to_f32(), -1.0);
        assert_eq!(i8::MIN.to_f32(), -1.0);
        assert_eq!(i32::MIN.to_f32(), -1.0);
        assert_eq!(0.25f32.to_f32(), 0.25);
    }

    #[test]
    fn downmixes_and_respects_output_size() {
        let stereo: [i16; 6] = [16_384, -16_384, 32_767, 32_767, 0, 16_384];
        let mut out = [9.0f32; 2];
        assert_eq!(downmix_into(&stereo, 2, &mut out), 2);
        assert!(out[0].abs() < 1e-4);
        assert!((out[1] - 1.0).abs() < 1e-3);
        let mut big = [0.0f32; 8];
        assert_eq!(downmix_into(&stereo, 2, &mut big), 3);
        assert!((big[2] - 0.25).abs() < 1e-4);
        // Mono passes through; a partial frame is ignored.
        assert_eq!(downmix_into(&[0.5f32, 0.25], 1, &mut big), 2);
        assert_eq!(downmix_into(&[1i16, 2, 3], 2, &mut big), 1);
    }
}
