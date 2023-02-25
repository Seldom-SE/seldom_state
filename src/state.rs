use std::{
    any::type_name,
    fmt::{self, Debug, Formatter},
};

use as_dyn_trait::as_dyn_trait;

use crate::{
    bundle::{Insert, Remove},
    prelude::*,
};

/// A state that an entity may be in. A state must implement [`Reflect`], but a workaround exists
/// for structs that contain types that do not implement [`Reflect`].
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

/// State that represents any state. Transitions from [`AnyState`] may transition
/// from any other state.
#[derive(Clone, Component, Debug, Reflect)]
pub struct AnyState;

#[as_dyn_trait]
pub(crate) trait DynState: 'static + Insert + Reflect + Remove + Send + Sync {
    fn dyn_clone(&self) -> Box<dyn DynState>;
}

impl Debug for dyn DynState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_reflect().fmt(f)
    }
}

impl<T: MachineState> DynState for T {
    fn dyn_clone(&self) -> Box<dyn DynState> {
        Box::new(self.clone())
    }
}

#[as_dyn_trait]
pub(crate) trait StateBuilderTyped<P: DynState, T, N: DynState>:
    Fn(&P, &T) -> Option<N> + Send + Sync
{
    fn dyn_clone(&self) -> Box<dyn StateBuilderTyped<P, T, N>>;
}

impl<B: 'static + Clone + Fn(&P, &T) -> Option<N> + Send + Sync, P: DynState, T, N: DynState>
    StateBuilderTyped<P, T, N> for B
{
    fn dyn_clone(&self) -> Box<dyn StateBuilderTyped<P, T, N>> {
        Box::new(self.clone())
    }
}

#[as_dyn_trait]
pub(crate) trait StateBuilder: Send + Sync {
    fn build(&self, state: &dyn DynState, result: &dyn Reflect) -> Option<Box<dyn DynState>>;
    fn dyn_clone(&self) -> Box<dyn StateBuilder>;
    fn debug(&self) -> String;
}

impl Debug for dyn StateBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.debug())
    }
}

impl<P: MachineState, T: Reflect, N: DynState> StateBuilder
    for Box<dyn StateBuilderTyped<P, T, N>>
{
    fn build(&self, state: &dyn DynState, result: &dyn Reflect) -> Option<Box<dyn DynState>> {
        self(
            state
                .as_reflect()
                .downcast_ref()
                .or(AnyState.as_reflect().downcast_ref())
                .unwrap(),
            result.downcast_ref().unwrap(),
        )
        .map(|state| Box::new(state).as_dyn_dyn_state())
    }

    fn dyn_clone(&self) -> Box<dyn StateBuilder> {
        Box::new(StateBuilderTyped::<P, T, N>::dyn_clone(&**self)).as_dyn_state_builder()
    }

    fn debug(&self) -> String {
        format!(
            "Fn({}, {}) -> {}",
            type_name::<&P>(),
            type_name::<T>(),
            type_name::<Option<N>>()
        )
    }
}
