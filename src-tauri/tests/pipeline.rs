//! Headless pipeline test: WAV fixtures go through the same code the app
//! runs (downmix, band-limited resample, Silero VAD segmenter with
//! padding, recogniser, join) with a fake audio source, and the word error
//! rate is checked.
//!
//! Needs a model: set LOCALYAPPER_TEST_MODEL_DIR (and optionally
//! LOCALYAPPER_TEST_MODEL_FAMILY) and run
//! `cargo test --test pipeline -- --include-ignored pipeline`.
//! CI does this with a cached model.

use localyapper_lib::core::audio::segmenter::{
    run_segmenter, EnergyDetector, Flags, SegmentSink, SileroDetector, SpeechDetector, VecSource,
};
use localyapper_lib::core::audio::wav;
use localyapper_lib::core::session::{join_segments, Event, SessionId};
use localyapper_lib::core::stt::engine::{Family, SherpaEngine};
use localyapper_lib::core::wer;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

#[derive(Default)]
struct Collect(Mutex<Vec<(usize, Vec<f32>)>>);

impl SegmentSink for Collect {
    fn push(&self, _id: SessionId, index: usize, samples: Vec<f32>) {
        if let Ok(mut v) = self.0.lock() {
            v.push((index, samples));
        }
    }
}

fn fixtures() -> Vec<(PathBuf, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut out: Vec<(PathBuf, String)> = std::fs::read_dir(&dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("wav"))
        .filter_map(|p| {
            let txt = std::fs::read_to_string(p.with_extension("txt")).ok()?;
            Some((p, txt.trim().to_string()))
        })
        .collect();
    out.sort();
    out
}

fn vad_model() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/silero_vad.onnx")
}

/// Run one clip through the segmenter and recogniser at its native rate.
fn transcribe_clip(engine: &SherpaEngine, path: &Path) -> (String, usize) {
    let data = wav::read_file(path).expect("read fixture");
    let mono = data.to_mono();
    let mut detector: Box<dyn SpeechDetector> = match SileroDetector::new(&vad_model()) {
        Ok(d) => Box::new(d),
        Err(e) => panic!("Silero VAD must load for the pipeline test: {e}"),
    };
    let sink = Collect::default();
    let stop = AtomicBool::new(true);
    let discard = AtomicBool::new(false);
    let ended = Mutex::new(None);
    run_segmenter(
        1,
        data.sample_rate,
        &mut VecSource::new(mono),
        Flags {
            stop: &stop,
            discard: &discard,
        },
        detector.as_mut(),
        &sink,
        &|ev| {
            if let Event::CaptureEnded { segments, .. } = ev {
                if let Ok(mut e) = ended.lock() {
                    *e = Some(segments);
                }
            }
        },
        &mut |_| {},
    )
    .expect("segmenter");
    let mut segs = sink.0.into_inner().unwrap_or_default();
    segs.sort_by_key(|(i, _)| *i);
    assert_eq!(ended.into_inner().ok().flatten(), Some(segs.len()));
    let texts: Vec<String> = segs
        .iter()
        .map(|(_, s)| engine.transcribe(s).expect("decode"))
        .collect();
    (join_segments(texts.iter().map(String::as_str)), segs.len())
}

#[test]
#[ignore = "needs LOCALYAPPER_TEST_MODEL_DIR (run with --include-ignored)"]
fn pipeline_fixtures_word_error_rate() {
    let dir = std::env::var_os("LOCALYAPPER_TEST_MODEL_DIR")
        .map(PathBuf::from)
        .expect("set LOCALYAPPER_TEST_MODEL_DIR to a model directory");
    let family = match std::env::var("LOCALYAPPER_TEST_MODEL_FAMILY") {
        Ok(f) => Family::parse(&f).expect("known family"),
        Err(_) => Family::detect(&dir).expect("detectable model family"),
    };
    let engine = SherpaEngine::load(family, &dir, 4).expect("load model");
    engine.warm_up();

    let clips = fixtures();
    assert!(clips.len() >= 5, "expected the LibriSpeech fixtures");
    let (mut errors, mut words) = (0usize, 0usize);
    for (path, reference) in &clips {
        let (text, segments) = transcribe_clip(&engine, path);
        let (e, w) = wer::counts(reference, &text);
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        println!(
            "{name}: {segments} segments, WER {:.1} %",
            100.0 * e as f64 / w.max(1) as f64
        );
        assert!(segments >= 1, "{name}: no speech segment");
        assert!(
            (e as f64) / (w.max(1) as f64) < 0.25,
            "{name}: WER too high"
        );
        if name.contains("pause") {
            assert!(segments >= 2, "the 3 s pause should split the clip");
        }
        errors += e;
        words += w;
    }
    let total = errors as f64 / words.max(1) as f64;
    println!("overall WER {:.2} % over {words} words", 100.0 * total);
    assert!(total < 0.08, "overall WER {:.2} %", 100.0 * total);
}

/// The energy fallback must also find speech in real recordings.
#[test]
fn pipeline_energy_fallback_finds_speech_in_fixtures() {
    let clips = fixtures();
    assert!(!clips.is_empty(), "fixtures are committed");
    for (path, _) in clips {
        let data = wav::read_file(&path).expect("read fixture");
        let sink = Collect::default();
        let stop = AtomicBool::new(true);
        let discard = AtomicBool::new(false);
        let mut vad = EnergyDetector::new();
        let summary = run_segmenter(
            1,
            data.sample_rate,
            &mut VecSource::new(data.to_mono()),
            Flags {
                stop: &stop,
                discard: &discard,
            },
            &mut vad,
            &sink,
            &|_| {},
            &mut |_| {},
        )
        .expect("segmenter");
        assert!(summary.segments >= 1, "{}: no segment", path.display());
        let expected_ms = data.duration_ms();
        assert!(
            summary.audio_ms.abs_diff(expected_ms) < 50,
            "{}: audio_ms {} vs {expected_ms}",
            path.display(),
            summary.audio_ms
        );
    }
}

/// The bundled Silero VAD loads and splits the fixture with a 3 s pause.
#[test]
fn pipeline_silero_vad_segments_fixtures() {
    for (path, _) in fixtures() {
        let data = wav::read_file(&path).expect("read fixture");
        let sink = Collect::default();
        let stop = AtomicBool::new(true);
        let discard = AtomicBool::new(false);
        let mut vad = SileroDetector::new(&vad_model()).expect("bundled Silero VAD loads");
        let summary = run_segmenter(
            1,
            data.sample_rate,
            &mut VecSource::new(data.to_mono()),
            Flags {
                stop: &stop,
                discard: &discard,
            },
            &mut vad,
            &sink,
            &|_| {},
            &mut |_| {},
        )
        .expect("segmenter");
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        assert!(summary.segments >= 1, "{name}: no segment");
        assert!(!summary.safety_net, "{name}: VAD found nothing");
        if name.contains("pause") {
            assert!(summary.segments >= 2, "{name}: pause not split");
        }
        // Silero's 15 s max_speech_duration is soft: past it the threshold
        // rises until the next pause, so a long unbroken sentence can run a
        // little over. Nothing may run away, though.
        for (_, s) in sink.0.into_inner().unwrap_or_default() {
            assert!(
                s.len() <= 16_000 * 20,
                "{name}: segment of {} samples",
                s.len()
            );
        }
    }
}
