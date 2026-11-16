//! Platform-independent core: no `tauri` imports anywhere in this tree.

pub mod audio;
pub mod download;
// Hand-formatted; kept out of rustfmt.
#[rustfmt::skip]
pub mod session;
pub mod stt;
pub mod wer;
