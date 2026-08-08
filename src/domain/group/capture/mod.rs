//! The capture concern: the atomic-entropy-set entity and the value objects
//! its evaluation produces.

mod entity;
mod report;

pub use entity::GroupCapture;
pub use report::{EntropySet, GroupReport, GroupStatus};
