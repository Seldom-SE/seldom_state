use std::fmt::Debug;

use bevy::ecs::system::EntityCommands;

use crate::prelude::*;

/// Represents a state that an entity may be in. A state must implement [`Reflect`],
/// but workarounds exist for structs that contain types that do not implement [`Reflect`]
/// and for enums.
///
/// For non-[`Reflect`] types:
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
/// And for enums:
///
/// ```rust
/// # use bevy::prelude::*;
/// #
/// #[derive(Clone)]
/// enum MyState {
///     Variant1,
///     Variant2,
/// }
///
/// #[derive(Clone, Component, Reflect)]
/// #[component(storage = "SparseSet")]
/// struct MyStateWrapper {
///     #[reflect(ignore)]
///     non_reflect_type: MyState
/// }
/// ```
///
/// These workarounds currently do not affect the functionality of your state machine.
///
/// If you are concerned with performance, I recommend having your states use sparse set storage,
/// since they may be added to and removed from entities.
pub trait MachineState: 'static + Clone + Component + Reflect + Send + Sync {}

impl<T: 'static + Clone + Component + Reflect + Send + Sync> MachineState for T {}

pub(crate) trait StateTransition: 'static + Reflect + Send + Sync {
    fn insert(&self, entity: &mut EntityCommands);
    fn remove(&self, entity: &mut EntityCommands);
    fn dyn_clone(&self) -> Box<dyn StateTransition>;
}

impl Debug for dyn StateTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_reflect().fmt(f)
    }
}

impl<T: MachineState> StateTransition for T {
    fn insert(&self, entity: &mut EntityCommands) {
        entity.insert(self.clone());
    }

    fn remove(&self, entity: &mut EntityCommands) {
        entity.remove::<T>();
    }

    fn dyn_clone(&self) -> Box<dyn StateTransition> {
        Box::new(self.clone())
    }
}
