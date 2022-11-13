use std::fmt::{Debug, Formatter, Result};

use crate::{
    bundle::{Insert, Remove},
    prelude::*,
};

/// Represents a state that an entity may be in. A state must implement [`Reflect`],
/// but a workaround exists for structs that contain types that do not implement [`Reflect`].
///
/// ```rust
/// # use bevy::prelude::*;
/// #
/// #[derive(Clone)]
/// struct NonReflectType;
///
/// #[derive(Clone, Component, Reflect)]
/// #[component(storage = "SparseSet")]
/// struct MyState {
///     #[reflect(ignore)]
///     non_reflect_type: NonReflectType
/// }
/// ```
///
/// This workaround currently does not affect the functionality of your state machine.
///
/// If you are concerned with performance, consider having your states use sparse set storage,
/// since they may be added to and removed from entities.
pub trait MachineState: 'static + Bundle + Clone + Reflect + Send + Sync {}

impl<T: 'static + Bundle + Clone + Reflect + Send + Sync> MachineState for T {}

pub(crate) trait StateTransition: 'static + Insert + Reflect + Remove + Send + Sync {
    fn dyn_clone(&self) -> Box<dyn StateTransition>;
}

impl Debug for dyn StateTransition {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.as_reflect().fmt(f)
    }
}

impl<T: MachineState> StateTransition for T {
    fn dyn_clone(&self) -> Box<dyn StateTransition> {
        Box::new(self.clone())
    }
}
