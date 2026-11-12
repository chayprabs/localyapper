//! sherpa-onnx offline recogniser wrapper.

use super::super::audio::wav::WavData;
use sherpa_onnx::{LinearResampler, OfflineRecognizer, OfflineRecognizerConfig};
use std::path::{Path, PathBuf};

pub const SAMPLE_RATE: i32 = 16_000;

/// Model families the catalog can describe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    NemoCtc,
    NemoTransducer,
    Moonshine,
}

impl Family {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "nemo_ctc" => Some(Self::NemoCtc),
            "nemo_transducer" | "transducer" => Some(Self::NemoTransducer),
            "moonshine" => Some(Self::Moonshine),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NemoCtc => "nemo_ctc",
            Self::NemoTransducer => "nemo_transducer",
            Self::Moonshine => "moonshine",
        }
    }

    /// Guess the family from the files present in `dir`.
    pub fn detect(dir: &Path) -> Option<Self> {
        let names = list_onnx(dir);
        let has = |needle: &str| names.iter().any(|n| n.contains(needle));
        if has("joiner") && has("encoder") {
            Some(Self::NemoTransducer)
        } else if has("decode") && (has("preprocess") || has("encode")) {
            Some(Self::Moonshine)
        } else if names.iter().any(|n| n.starts_with("model")) {
            Some(Self::NemoCtc)
        } else {
            None
        }
    }
}

fn list_onnx(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .filter(|n| n.ends_with(".onnx"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The exact files a family needs. Paths are absolute.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModelFiles {
    pub tokens: PathBuf,
    pub model: Option<PathBuf>,
    pub encoder: Option<PathBuf>,
    pub decoder: Option<PathBuf>,
    pub joiner: Option<PathBuf>,
    pub preprocessor: Option<PathBuf>,
    pub uncached_decoder: Option<PathBuf>,
    pub cached_decoder: Option<PathBuf>,
    pub merged_decoder: Option<PathBuf>,
}

impl ModelFiles {
    /// Build from a list of file names (as stored in the manifest).
    pub fn from_names(family: Family, dir: &Path, names: &[&str]) -> Result<Self, String> {
        let pick = |pred: &dyn Fn(&str) -> bool| -> Option<PathBuf> {
            // Prefer int8 builds when several files match.
            let mut matches: Vec<&&str> = names.iter().filter(|n| pred(n)).collect();
            matches.sort_by_key(|n| !n.contains("int8"));
            matches.first().map(|n| dir.join(n))
        };
        let onnx = |n: &str| n.ends_with(".onnx");
        let tokens = names
            .iter()
            .find(|n| n.ends_with("tokens.txt"))
            .map(|n| dir.join(n))
            .ok_or("tokens.txt is missing")?;
        let mut f = ModelFiles {
            tokens,
            ..Default::default()
        };
        match family {
            Family::NemoCtc => {
                f.model = pick(&|n| onnx(n) && n.starts_with("model"));
                if f.model.is_none() {
                    return Err("CTC model file is missing".into());
                }
            }
            Family::NemoTransducer => {
                f.encoder = pick(&|n| onnx(n) && n.contains("encoder"));
                f.decoder = pick(&|n| onnx(n) && n.contains("decoder"));
                f.joiner = pick(&|n| onnx(n) && n.contains("joiner"));
                if f.encoder.is_none() || f.decoder.is_none() || f.joiner.is_none() {
                    return Err("transducer needs encoder, decoder and joiner".into());
                }
            }
            Family::Moonshine => {
                f.preprocessor = pick(&|n| onnx(n) && n.contains("preprocess"));
                f.encoder = pick(&|n| onnx(n) && n.contains("encode") && !n.contains("decode"));
                f.uncached_decoder = pick(&|n| onnx(n) && n.contains("uncached_decode"));
                f.cached_decoder =
                    pick(&|n| onnx(n) && n.contains("cached_decode") && !n.contains("uncached"));
                f.merged_decoder = pick(&|n| onnx(n) && n.contains("merged_decode"));
                let v1 = f.preprocessor.is_some()
                    && f.uncached_decoder.is_some()
                    && f.cached_decoder.is_some();
                let v2 = f.merged_decoder.is_some();
                if f.encoder.is_none() || !(v1 || v2) {
                    return Err("Moonshine model files are incomplete".into());
                }
            }
        }
        Ok(f)
    }

    /// Build from whatever files are in `dir` (legacy installs, bench).
    pub fn discover(family: Family, dir: &Path) -> Result<Self, String> {
        let mut names = list_onnx(dir);
        if dir.join("tokens.txt").is_file() {
            names.push("tokens.txt".into());
        }
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        Self::from_names(family, dir, &refs)
    }
}

fn s(p: &Option<PathBuf>) -> Option<String> {
    p.as_ref().map(|p| p.to_string_lossy().into_owned())
}

/// Recogniser configuration for one family. No `blank_penalty`: the
/// library default is the tested one.
pub fn recognizer_config(
    family: Family,
    files: &ModelFiles,
    threads: i32,
) -> OfflineRecognizerConfig {
    let mut cfg = OfflineRecognizerConfig::default();
    cfg.model_config.num_threads = threads.max(1);
    cfg.model_config.provider = Some("cpu".into());
    cfg.model_config.tokens = Some(files.tokens.to_string_lossy().into_owned());
    cfg.decoding_method = Some("greedy_search".into());
    match family {
        Family::NemoCtc => cfg.model_config.nemo_ctc.model = s(&files.model),
        Family::NemoTransducer => {
            cfg.model_config.transducer.encoder = s(&files.encoder);
            cfg.model_config.transducer.decoder = s(&files.decoder);
            cfg.model_config.transducer.joiner = s(&files.joiner);
            cfg.model_config.model_type = Some("nemo_transducer".into());
        }
        Family::Moonshine => {
            let m = &mut cfg.model_config.moonshine;
            m.preprocessor = s(&files.preprocessor);
            m.encoder = s(&files.encoder);
            m.uncached_decoder = s(&files.uncached_decoder);
            m.cached_decoder = s(&files.cached_decoder);
            m.merged_decoder = s(&files.merged_decoder);
        }
    }
    cfg
}

/// A loaded recogniser. `OfflineRecognizer` is `Send + Sync` in the crate.
pub struct SherpaEngine {
    recognizer: OfflineRecognizer,
    family: Family,
}

impl SherpaEngine {
    pub fn load(family: Family, dir: &Path, threads: i32) -> Result<Self, String> {
        let files = ModelFiles::discover(family, dir)?;
        Self::load_files(family, &files, threads)
    }

    pub fn load_files(family: Family, files: &ModelFiles, threads: i32) -> Result<Self, String> {
        for p in [
            Some(&files.tokens),
            files.model.as_ref(),
            files.encoder.as_ref(),
            files.decoder.as_ref(),
            files.joiner.as_ref(),
            files.preprocessor.as_ref(),
            files.uncached_decoder.as_ref(),
            files.cached_decoder.as_ref(),
            files.merged_decoder.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !p.is_file() {
                return Err(format!("model file missing: {}", p.display()));
            }
        }
        let cfg = recognizer_config(family, files, threads);
        let recognizer = OfflineRecognizer::create(&cfg)
            .ok_or_else(|| format!("sherpa-onnx could not load the {} model", family.as_str()))?;
        Ok(Self { recognizer, family })
    }

    pub fn family(&self) -> Family {
        self.family
    }

    /// Decode 0.5 s of silence once so the first real decode is not cold.
    pub fn warm_up(&self) {
        let silence = vec![0.0f32; 8_000];
        let _ = self.decode(&silence);
    }

    fn decode(&self, samples: &[f32]) -> Result<String, String> {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(SAMPLE_RATE, samples);
        self.recognizer.decode(&stream);
        let result = stream.get_result().ok_or("no result from recogniser")?;
        Ok(result.text.trim().to_string())
    }

    /// Transcribe 16 kHz mono samples. Clips under 0.1 s return "".
    pub fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
        if samples.len() < 1_600 {
            return Ok(String::new());
        }
        self.decode(samples)
    }

    /// Downmix and resample a decoded WAV to 16 kHz mono.
    pub fn to_16k_mono(data: &WavData) -> Result<Vec<f32>, String> {
        let mono = data.to_mono();
        if data.sample_rate == SAMPLE_RATE as u32 {
            return Ok(mono);
        }
        let r = LinearResampler::create(data.sample_rate as i32, SAMPLE_RATE)
            .ok_or("could not create resampler")?;
        Ok(r.resample(&mono, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_int8_and_family_files() {
        let d = Path::new("/m");
        let f = ModelFiles::from_names(
            Family::NemoCtc,
            d,
            &["model.onnx", "model.int8.onnx", "tokens.txt"],
        )
        .expect("ctc");
        assert_eq!(f.model, Some(d.join("model.int8.onnx")));

        let f = ModelFiles::from_names(
            Family::NemoTransducer,
            d,
            &[
                "encoder.int8.onnx",
                "decoder.int8.onnx",
                "joiner.int8.onnx",
                "tokens.txt",
            ],
        )
        .expect("tdt");
        assert_eq!(f.joiner, Some(d.join("joiner.int8.onnx")));

        let f = ModelFiles::from_names(
            Family::Moonshine,
            d,
            &[
                "preprocess.onnx",
                "encode.int8.onnx",
                "uncached_decode.int8.onnx",
                "cached_decode.int8.onnx",
                "tokens.txt",
            ],
        )
        .expect("moonshine");
        assert_eq!(f.cached_decoder, Some(d.join("cached_decode.int8.onnx")));
        assert_eq!(
            f.uncached_decoder,
            Some(d.join("uncached_decode.int8.onnx"))
        );
        assert_eq!(f.encoder, Some(d.join("encode.int8.onnx")));
    }

    #[test]
    fn missing_files_are_errors() {
        let d = Path::new("/m");
        assert!(ModelFiles::from_names(Family::NemoCtc, d, &["tokens.txt"]).is_err());
        assert!(ModelFiles::from_names(Family::NemoCtc, d, &["model.onnx"]).is_err());
        assert!(
            ModelFiles::from_names(Family::NemoTransducer, d, &["encoder.onnx", "tokens.txt"])
                .is_err()
        );
    }

    #[test]
    fn family_names_round_trip() {
        for f in [Family::NemoCtc, Family::NemoTransducer, Family::Moonshine] {
            assert_eq!(Family::parse(f.as_str()), Some(f));
        }
        assert_eq!(Family::parse("transducer"), Some(Family::NemoTransducer));
        assert_eq!(Family::parse("whisper"), None);
    }
}
