//! Platform-independent core: no `tauri` imports anywhere in this tree.

pub mod audio;
pub mod download;
// Appendix A of docs/REBUILD_PLAN.md, kept byte-for-byte.
#[rustfmt::skip]
pub mod session;
pub mod stt;
pub mod wer;
