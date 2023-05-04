//! Bevy system set stuff

use crate::prelude::*;

/// System sets used by this crate
#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemSet)]
pub enum StateSet {
    /// Do state transitions
    Transition,
}
