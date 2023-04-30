//! Bevy system set stuff

use crate::prelude::*;

/// System sets used by this crate
#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum StateSet {
    /// Test for triggers
    Trigger,
    /// Do state transitions
    Transition,
}
