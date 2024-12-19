//! Component-based state machine plugin for Bevy. Useful for AI, player state, and other entities
//! that occupy different states.

#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod machine;
pub mod set;
mod state;
pub mod trigger;

use prelude::*;

/// Add to your app to use this crate
#[derive(Debug, Default)]
pub struct StateMachinePlugin;

impl Plugin for StateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((machine::plug, trigger::plug));
    }
}

/// Module for convenient imports. Use with `use seldom_state::prelude::*;`.
pub mod prelude {
    pub(crate) use bevy::prelude::*;
    #[cfg(feature = "leafwing_input")]
    pub(crate) use leafwing_input_manager::prelude::*;

    #[cfg(feature = "leafwing_input")]
    pub use crate::trigger::{
        action_data, axis_pair, axis_pair_length_bounds, axis_pair_max_length,
        axis_pair_min_length, axis_pair_rotation_bounds, axis_pair_unbounded, clamped_axis_pair,
        clamped_axis_pair_length_bounds, clamped_axis_pair_max_length,
        clamped_axis_pair_min_length, clamped_axis_pair_rotation_bounds,
        clamped_axis_pair_unbounded, clamped_value, clamped_value_max, clamped_value_min,
        clamped_value_unbounded, just_pressed, just_released, pressed, value, value_max, value_min,
        value_unbounded,
    };
    pub use crate::{
        machine::StateMachine,
        state::{AnyState, EntityState},
        trigger::{always, done, on_event, Done, EntityTrigger, IntoTrigger, Never},
        StateMachinePlugin,
    };
}
