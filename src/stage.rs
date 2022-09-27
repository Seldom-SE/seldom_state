//! Bevy stage stuff

use crate::prelude::*;

/// Stages used by this crate
#[derive(Debug, StageLabel)]
pub enum StateStage {
    /// Test for triggers
    Trigger,
    /// Do state transitions
    Transition,
}
