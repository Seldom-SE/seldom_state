//! Component-based state machine plugin for Bevy. Useful for AI, player state,
//! and other entities that occupy different states.

#![warn(missing_docs)]

mod machine;
pub mod set;
mod state;
mod trigger;

use machine::transition_system;
use prelude::*;
use set::StateSet;
use trigger::remove_done_markers;

/// Add to your app to use this crate.
#[derive(Debug, Default)]
pub struct StateMachinePlugin;

impl Plugin for StateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.fn_plugin(state_machine_plugin);
    }
}

/// Function called by [`StateMachinePlugin`]. You may instead call it directly
/// or use `seldom_fn_plugin`, which is another crate I maintain.
pub fn state_machine_plugin(app: &mut App) {
    app.configure_set(StateSet::Transition.in_base_set(CoreSet::PostUpdate))
        .add_system(transition_system.in_set(StateSet::Transition))
        .add_system(remove_done_markers.in_set(StateSet::Transition));
}

/// Module for convenient imports. Use with `use seldom_state::prelude::*;`.
pub mod prelude {
    pub(crate) use bevy::prelude::*;
    #[cfg(feature = "leafwing_input")]
    pub(crate) use leafwing_input_manager::prelude::*;
    pub(crate) use seldom_fn_plugin::FnPluginExt;

    #[cfg(feature = "leafwing_input")]
    pub use crate::trigger::{
        input_trigger_plugin, ActionDataTrigger, AxisPairTrigger, ClampedAxisPairTrigger,
        ClampedValueTrigger, InputTriggerPlugin, JustPressedTrigger, JustReleasedTrigger,
        PressedTrigger, ReleasedTrigger, ValueTrigger,
    };
    pub use crate::{
        machine::StateMachine,
        state::{AnyState, MachineState},
        state_machine_plugin,
        trigger::{
            AlwaysTrigger, BoolTrigger, Done, DoneTrigger, Never, NotTrigger, OptionTrigger,
            Trigger,
        },
        StateMachinePlugin,
    };
}
