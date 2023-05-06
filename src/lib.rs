//! Component-based state machine plugin for Bevy. Useful for AI, player state,
//! and other entities that occupy different states.

#![warn(missing_docs)]

mod machine;
pub mod set;
mod state;
mod trigger;

use machine::machine_plugin;
use prelude::*;
use trigger::trigger_plugin;

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
    app.fn_plugin(machine_plugin).fn_plugin(trigger_plugin);
}

/// Module for convenient imports. Use with `use seldom_state::prelude::*;`.
pub mod prelude {
    pub(crate) use bevy::prelude::*;
    #[cfg(feature = "leafwing_input")]
    pub(crate) use leafwing_input_manager::prelude::*;
    pub(crate) use seldom_fn_plugin::FnPluginExt;

    #[cfg(feature = "leafwing_input")]
    pub use crate::trigger::{
        ActionDataTrigger, AxisPairTrigger, ClampedAxisPairTrigger, ClampedValueTrigger,
        JustPressedTrigger, JustReleasedTrigger, PressedTrigger, ReleasedTrigger, ValueTrigger,
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
