//! Component-based state machine plugin for Bevy. Useful for AI, player state, and other entities
//! that occupy different states.

#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

pub mod machine;
pub mod set;
mod state;
pub mod trigger;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{intern::Interned, schedule::ScheduleLabel};
use prelude::*;

/// Add to your app to use this crate
#[derive(Debug)]
pub struct StateMachinePlugin {
    schedule: Interned<dyn ScheduleLabel>,
}

impl Default for StateMachinePlugin {
    fn default() -> Self {
        Self {
            schedule: PostUpdate.intern(),
        }
    }
}

impl StateMachinePlugin {
    /// Sets the schedule in which `StateMachine`s are updated. Defaults to `PostUpdate`.
    pub fn schedule(mut self, schedule: impl ScheduleLabel) -> Self {
        self.schedule = schedule.intern();
        self
    }
}

impl Plugin for StateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((machine::plug(self.schedule), trigger::plug(self.schedule)));
    }
}

/// Module for convenient imports. Use with `use seldom_state::prelude::*;`.
pub mod prelude {
    pub(crate) use bevy_app::prelude::*;
    pub(crate) use bevy_ecs::prelude::*;
    pub(crate) use bevy_log::prelude::*;
    #[allow(unused)]
    pub(crate) use bevy_math::prelude::*;
    pub(crate) use bevy_utils::prelude::*;
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
        machine::{StateMachine, Trans},
        state::{AnyState, EntityState, NotState, OneOfState},
        trigger::{always, done, on_message, Done, EntityTrigger, IntoTrigger, Never},
        StateMachinePlugin,
    };
}

const OK: Result = Ok(());

#[derive(Deref, DerefMut, Default)]
struct ErrList(Vec<BevyError>);

impl ErrList {
    fn push<T, E: Into<BevyError>>(&mut self, res: Result<T, E>) -> Option<T> {
        match res {
            Ok(t) => Some(t),
            Err(err) => {
                self.0.push(err.into());
                None
            }
        }
    }
}

impl From<ErrList> for Result {
    fn from(value: ErrList) -> Self {
        if value.is_empty() {
            OK
        } else {
            Err(value
                .iter()
                .map(BevyError::to_string)
                .collect::<Vec<_>>()
                .join("; ")
                .into())
        }
    }
}
