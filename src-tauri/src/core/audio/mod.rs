//! Audio helpers that do not touch a device.

pub mod convert;
// Appendix B of docs/REBUILD_PLAN.md, kept byte-for-byte.
#[rustfmt::skip]
pub mod rolling;
pub mod wav;
