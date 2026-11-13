//! Speech model bench harness.
//!
//! ```text
//! cargo run --release --example bench -- \
//!     --model-id parakeet-110m --model-dir /path/to/model \
//!     --fixtures src-tauri/tests/fixtures --threads 4 [--family nemo_ctc]
//! ```
//!
//! Each clip goes through the same path as the app: Silero VAD segments
//! (src-tauri/resources/silero_vad.onnx, or --vad PATH) decoded one by one
//! and joined. Prints one Markdown table row per clip and a summary row
//! with load ms, warm-up ms, real-time factor, peak resident memory and
//! word error rate. Run it once per thread count so peak memory is per
//! configuration.

use localyapper_lib::core::audio::segmenter::{
    run_segmenter, Flags, SegmentSink, SileroDetector, VecSource,
};
use localyapper_lib::core::audio::wav;
use localyapper_lib::core::session::{join_segments, SessionId};
use localyapper_lib::core::stt::engine::{Family, SherpaEngine};
use localyapper_lib::core::wer;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
struct Collect(Mutex<Vec<(usize, Vec<f32>)>>);

impl SegmentSink for Collect {
    fn push(&self, _id: SessionId, index: usize, samples: Vec<f32>) {
        if let Ok(mut v) = self.0.lock() {
            v.push((index, samples));
        }
    }
}

/// Cut a clip into VAD segments at its native rate, as the app does.
fn segments(data: &wav::WavData, vad_model: &Path) -> Result<Vec<Vec<f32>>, String> {
    let mut vad = SileroDetector::new(vad_model)?;
    let sink = Collect::default();
    let stop = AtomicBool::new(true);
    let discard = AtomicBool::new(false);
    run_segmenter(
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
    )?;
    let mut segs = sink.0.into_inner().unwrap_or_default();
    segs.sort_by_key(|(i, _)| *i);
    Ok(segs.into_iter().map(|(_, s)| s).collect())
}

struct Args {
    model_id: String,
    model_dir: PathBuf,
    fixtures: PathBuf,
    threads: i32,
    family: Option<String>,
    vad: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut model_id = None;
    let mut model_dir = None;
    let mut fixtures = None;
    let mut threads = 4;
    let mut family = None;
    let mut vad = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/silero_vad.onnx");
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("missing value for {a}"));
        match a.as_str() {
            "--model-id" => model_id = Some(val()?),
            "--model-dir" => model_dir = Some(PathBuf::from(val()?)),
            "--fixtures" => fixtures = Some(PathBuf::from(val()?)),
            "--threads" => threads = val()?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--family" => family = Some(val()?),
            "--vad" => vad = PathBuf::from(val()?),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Args {
        model_id: model_id.ok_or("--model-id is required")?,
        model_dir: model_dir.ok_or("--model-dir is required")?,
        fixtures: fixtures.ok_or("--fixtures is required")?,
        threads,
        family,
        vad,
    })
}

/// Peak resident memory of this process in MB, where the OS exposes it
/// cheaply. The bench runs on the Linux runner.
fn peak_rss_mb() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kb: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024.0)
}

fn clips(dir: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("wav") {
            continue;
        }
        let txt = p.with_extension("txt");
        if let Ok(reference) = std::fs::read_to_string(&txt) {
            out.push((p, reference.trim().to_string()));
        }
    }
    out.sort();
    Ok(out)
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let family = match &args.family {
        Some(f) => Family::parse(f).ok_or_else(|| format!("unknown family {f}"))?,
        None => {
            Family::detect(&args.model_dir).ok_or("cannot detect model family; pass --family")?
        }
    };

    let t = Instant::now();
    let engine = SherpaEngine::load(family, &args.model_dir, args.threads)?;
    let load_ms = t.elapsed().as_millis();
    let t = Instant::now();
    engine.warm_up();
    let warm_ms = t.elapsed().as_millis();

    let mut total_err = 0usize;
    let mut total_words = 0usize;
    let mut total_audio = 0.0f64;
    let mut total_decode = 0.0f64;
    println!("| model | threads | clip | audio s | decode s | RTF | WER % |");
    println!("|---|---|---|---|---|---|---|");
    for (path, reference) in clips(&args.fixtures)? {
        let data = wav::read_file(&path)?;
        let audio_s = data.duration_ms() as f64 / 1000.0;
        let segs = segments(&data, &args.vad)?;
        let t = Instant::now();
        let mut texts = Vec::with_capacity(segs.len());
        for seg in &segs {
            texts.push(engine.transcribe(seg)?);
        }
        let dec = t.elapsed().as_secs_f64();
        let text = join_segments(texts.iter().map(String::as_str));
        let (err, words) = wer::counts(&reference, &text);
        total_err += err;
        total_words += words;
        total_audio += audio_s;
        total_decode += dec;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        println!(
            "| {} | {} | {} | {:.1} | {:.2} | {:.3} | {:.1} |",
            args.model_id,
            args.threads,
            name,
            audio_s,
            dec,
            dec / audio_s.max(1e-9),
            100.0 * err as f64 / words.max(1) as f64
        );
    }
    let rtf = total_decode / total_audio.max(1e-9);
    let wer_pct = 100.0 * total_err as f64 / total_words.max(1) as f64;
    let peak = peak_rss_mb().map_or("n/a".to_string(), |v| format!("{v:.0}"));
    println!();
    println!(
        "SUMMARY | {} | threads {} | load_ms {} | warmup_ms {} | rtf {:.3} | peak_rss_mb {} | wer_pct {:.2}",
        args.model_id, args.threads, load_ms, warm_ms, rtf, peak, wer_pct
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("bench: {e}");
        std::process::exit(1);
    }
}
