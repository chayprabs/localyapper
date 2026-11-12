//! P4 gate: a fresh profile downloads a model from the pinned manifest,
//! verifies it, and dictates a fixture with it.
//!
//! Needs network access to huggingface.co, so it is ignored unless run
//! with `--include-ignored` (CI does this in the pipeline job).

use localyapper_lib::core::audio::segmenter::{
    run_segmenter, Flags, SegmentSink, SileroDetector, VecSource,
};
use localyapper_lib::core::audio::wav;
use localyapper_lib::core::download::{Downloader, FileSpec};
use localyapper_lib::core::session::{join_segments, SessionId};
use localyapper_lib::core::stt::catalog;
use localyapper_lib::core::stt::engine::SherpaEngine;
use localyapper_lib::core::wer;
use std::path::Path;
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

#[test]
#[ignore = "downloads a model from huggingface.co (run with --include-ignored)"]
fn fresh_profile_downloads_verifies_and_dictates() {
    let id = "moonshine-tiny-en";
    let manifest = catalog::manifest().expect("manifest");
    let entry = manifest.models.get(id).expect("model in catalog");
    let models = std::env::temp_dir().join(format!("ly-fresh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&models);
    let dir = catalog::model_dir(&models, id);

    // Download and verify, as the Speech page does.
    let files: Vec<FileSpec> = entry
        .files
        .iter()
        .map(|f| FileSpec {
            url: f.url.clone(),
            size: f.size,
            sha256: f.sha256.clone(),
            dest: dir.join(&f.name),
        })
        .collect();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let last = Mutex::new((0u64, 0u64));
    rt.block_on(Downloader::new().expect("client").fetch_all(
        &files,
        &AtomicBool::new(false),
        &|p| {
            if let Ok(mut l) = last.lock() {
                *l = (p.downloaded, p.total);
            }
        },
    ))
    .expect("download");
    let (done, total) = last.into_inner().unwrap_or_default();
    assert_eq!(done, total);
    assert_eq!(total, entry.total_size());
    std::fs::write(
        dir.join(catalog::VERIFIED_MARKER),
        catalog::marker_text(entry),
    )
    .expect("marker");
    assert!(catalog::is_installed(&dir, entry));

    // Resolve, load and dictate a fixture through the segmenter.
    let spec = catalog::spec(&models, id, &manifest).expect("spec");
    let engine = SherpaEngine::load_files(spec.family, &spec.files, 2).expect("load");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let data = wav::read_file(&fixtures.join("clip01_16k.wav")).expect("wav");
    let want = std::fs::read_to_string(fixtures.join("clip01_16k.txt")).expect("txt");
    let mut vad = SileroDetector::new(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/silero_vad.onnx"),
    )
    .expect("vad");
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
    )
    .expect("segmenter");
    let mut segs = sink.0.into_inner().unwrap_or_default();
    segs.sort_by_key(|(i, _)| *i);
    let texts: Vec<String> = segs
        .iter()
        .map(|(_, s)| engine.transcribe(s).expect("decode"))
        .collect();
    let got = join_segments(texts.iter().map(String::as_str));
    let w = wer::wer(&want, &got);
    println!("{id}: {got:?} WER {:.1} %", 100.0 * w);
    assert!(w < 0.25, "WER {:.1} %", 100.0 * w);
    let _ = std::fs::remove_dir_all(&models);
}
