//! Component-based state machine plugin for Bevy. Useful for AI, player state,
//! and other entities that occupy different states.

#![warn(missing_docs)]

mod machine;
pub mod stage;
mod state;
mod trigger;

use machine::machine_plugin;
use prelude::*;
use stage::StateStage;

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
    app.add_stage_after(
        CoreStage::Update,
        StateStage::Trigger,
        SystemStage::parallel(),
    )
    .add_stage_after(
        StateStage::Trigger,
        StateStage::Transition,
        SystemStage::parallel(),
    )
    .fn_plugin(trigger_plugin::<AlwaysTrigger>)
    .fn_plugin(trigger_plugin::<DoneTrigger>)
    .fn_plugin(machine_plugin);
}

/// Module for convenient imports. Use with `use seldom_state::prelude::*;`.
pub mod prelude {
    pub(crate) use bevy::prelude::*;
    pub(crate) use seldom_fn_plugin::FnPluginExt;

    pub use crate::{
        machine::StateMachine,
        state::MachineState,
        state_machine_plugin,
        trigger::{
            trigger_plugin, AlwaysTrigger, Done, DoneTrigger, NotTrigger, Trigger, TriggerPlugin,
        },
        StateMachinePlugin,
    };
}
