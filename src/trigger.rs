#[cfg(feature = "leafwing_input")]
mod input;

#[cfg(feature = "leafwing_input")]
pub use input::{
    input_trigger_plugin, ActionDataTrigger, AxisPairTrigger, ClampedAxisPairTrigger,
    ClampedValueTrigger, InputTriggerPlugin, JustPressedTrigger, JustReleasedTrigger,
    PressedTrigger, ReleasedTrigger, ValueTrigger,
};

use std::{
    any::TypeId,
    convert::Infallible,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamFetch};

use crate::{prelude::*, stage::StateStage};

/// Plugin that must be added for a trigger to be checked. Also registers the [`NotTrigger<T>`]
/// trigger.
///
/// # Panics
///
/// Panics with a system param conflict if the given trigger can access [`StateMachine`]
/// as a parameter
#[derive(Debug)]
pub struct TriggerPlugin<T: Trigger>(PhantomData<T>);

impl<T: Trigger> Plugin for TriggerPlugin<T> {
    fn build(&self, app: &mut App) {
        app.fn_plugin(trigger_plugin::<T>);
    }
}

impl<T: Trigger> Default for TriggerPlugin<T> {
    fn default() -> Self {
        Self(default())
    }
}

/// Function called by [`TriggerPlugin`]. You may instead call it directly
/// or use `seldom_fn_plugin`, which is another crate I maintain.
pub fn trigger_plugin<T: Trigger>(app: &mut App) {
    app.add_system_to_stage(StateStage::Trigger, check_trigger::<T>)
        .add_system_to_stage(StateStage::Trigger, check_trigger::<NotTrigger<T>>)
        .add_startup_system(register_trigger::<T>)
        .add_startup_system(register_trigger::<NotTrigger<T>>);
}

pub(crate) fn trigger_plugin_internal(app: &mut App) {
    app.fn_plugin(trigger_plugin::<AlwaysTrigger>)
        .fn_plugin(trigger_plugin::<DoneTrigger>)
        .init_resource::<RegisteredTriggers>()
        .add_system(validate_triggers)
        .add_system_to_stage(StateStage::Transition, remove_done_markers);
}

/// Wrapper for [`core::convert::Infallible`]. Use for [`Trigger::Err`] if the trigger
/// is infallible.
#[derive(Debug, Deref, DerefMut, Reflect)]
pub struct Never {
    #[reflect(ignore)]
    never: Infallible,
}

/// Types that implement this may be used in [`StateMachine`]s to transition from one state
/// to another. [`TriggerPlugin`] must be added for each trigger. Look at an example
/// for implementing this trait, since it can be tricky.
pub trait Trigger: 'static + Clone + Reflect + Send + Sync {
    /// System parameter provided to [`Trigger::trigger`]. Must not access [`StateMachine`].
    type Param<'w, 's>: SystemParam;
    /// When the trigger occurs, this data is returned from `trigger`, and passed
    /// to every transition builder on this trigger. If there's no relevant information to pass,
    /// just use `()`.
    type Ok: Reflect;
    /// When the trigger does not occur, this data is returned from `trigger`. In this case,
    /// [`NotTrigger<Self>`] passes it to every transition builder on this trigger.
    /// If there's no relevant information to pass, just use `()`. If this trigger is infallible,
    /// use [`Never`].
    type Err: Reflect;

    /// Called for every entity that may transition to a state on this trigger. Return `true`
    /// if it should transition, and `false` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as Trigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> Result<Self::Ok, Self::Err>;

    /// Get the name of the type, for use in logging. You probably should not override this.
    fn base_type_name(&self) -> &str {
        self.type_name()
    }
}

/// Automatically implements [`Trigger`]. Implement this instead of there is no relevant
/// information to pass for [`Trigger::Err`].
pub trait OptionTrigger: 'static + Clone + Reflect + Send + Sync {
    /// System parameter provided to [`OptionTrigger::trigger`]. Must not access [`StateMachine`].
    type Param<'w, 's>: SystemParam;
    /// When the trigger occurs, this data is returned from `trigger`, and passed
    /// to every transition builder on this trigger. If there's no relevant information to pass,
    /// just use `()`.
    type Some: Reflect;

    /// Called for every entity that may transition to a state on this trigger. Return `true`
    /// if it should transition, and `false` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as OptionTrigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> Option<Self::Some>;
}

impl<T: OptionTrigger> Trigger for T {
    type Param<'w, 's> = <Self as OptionTrigger>::Param<'w, 's>;
    type Ok = <Self as OptionTrigger>::Some;
    type Err = ();

    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as Trigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> Result<Self::Ok, ()> {
        OptionTrigger::trigger(self, entity, param).ok_or(())
    }
}

/// Automatically implements [`Trigger`]. Implement this instead of there is no relevant
/// information to pass for [`Trigger::Ok`] or [`Trigger::Err`].
pub trait BoolTrigger: 'static + Clone + Reflect + Send + Sync {
    /// System parameter provided to [`BoolTrigger::trigger`]. Must not access [`StateMachine`].
    type Param<'w, 's>: SystemParam;

    /// Called for every entity that may transition to a state on this trigger. Return `true`
    /// if it should transition, and `false` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as BoolTrigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> bool;
}

impl<T: BoolTrigger> OptionTrigger for T {
    type Param<'w, 's> = <Self as BoolTrigger>::Param<'w, 's>;
    type Some = ();

    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as Trigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> Option<()> {
        BoolTrigger::trigger(self, entity, param).then_some(())
    }
}

/// Trigger that always transitions
#[derive(Clone, Debug, Reflect)]
pub struct AlwaysTrigger;

impl Trigger for AlwaysTrigger {
    type Param<'w, 's> = ();
    type Ok = ();
    type Err = Never;

    fn trigger(&self, _: Entity, _: &()) -> Result<(), Never> {
        Ok(())
    }
}

/// Trigger that negates the contained trigger. It is works for any trigger that is added
/// by [`TriggerPlugin`].
#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct NotTrigger<T: Trigger>(pub T);

impl<T: Trigger> Trigger for NotTrigger<T> {
    type Param<'w, 's> = T::Param<'w, 's>;
    type Ok = T::Err;
    type Err = T::Ok;

    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as Trigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> Result<T::Err, T::Ok> {
        let Self(trigger) = self;
        match trigger.trigger(entity, param) {
            Ok(ok) => Err(ok),
            Err(err) => Ok(err),
        }
    }

    fn base_type_name(&self) -> &str {
        let Self(trigger) = self;
        trigger.base_type_name()
    }
}

/// Marker component that represents that the current state has completed. Removed
/// from every entity each frame after checking triggers. To be used with [`DoneTrigger`].
#[derive(Component, Debug, Eq, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
pub enum Done {
    /// Success variant
    Success,
    /// Failure variant
    Failure,
}

/// Trigger that transitions if the entity has the [`Done`] component
/// with the associated variant.
#[derive(Clone, Debug, Reflect)]
pub enum DoneTrigger {
    /// Success variant
    Success,
    /// Failure variant
    Failure,
}

impl BoolTrigger for DoneTrigger {
    type Param<'w, 's> = Query<'w, 's, &'static Done>;

    fn trigger(&self, entity: Entity, param: &Self::Param<'_, '_>) -> bool {
        param
            .get(entity)
            .map(|done| self.as_done() == *done)
            .unwrap_or(false)
    }
}

impl DoneTrigger {
    fn as_done(&self) -> Done {
        match self {
            Self::Success => Done::Success,
            Self::Failure => Done::Failure,
        }
    }
}

pub(crate) trait DynTrigger: Reflect {
    fn dyn_clone(&self) -> Box<dyn DynTrigger>;
}

impl Debug for dyn DynTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_reflect().fmt(f)
    }
}

impl<T: Trigger> DynTrigger for T {
    fn dyn_clone(&self) -> Box<dyn DynTrigger> {
        Box::new(self.clone())
    }
}

#[derive(Default, Deref, DerefMut, Resource)]
pub(crate) struct RegisteredTriggers(Vec<TypeId>);

fn register_trigger<T: Trigger>(mut registered: ResMut<RegisteredTriggers>) {
    registered.push(TypeId::of::<T>())
}

fn validate_triggers(
    machines: Query<&StateMachine, Added<StateMachine>>,
    registered: Res<RegisteredTriggers>,
) {
    for machine in &machines {
        machine.validate_triggers(&registered)
    }
}

fn check_trigger<T: Trigger>(
    mut machines: Query<(Entity, &mut StateMachine)>,
    param: StaticSystemParam<T::Param<'_, '_>>,
) {
    for (entity, mut machine) in &mut machines {
        let mut marks = Vec::default();

        for (i, trigger) in machine.get_triggers::<T>(false).into_iter().enumerate() {
            if let Ok(result) = trigger.trigger(entity, &param) {
                marks.push((i, result));
            }
        }

        for (i, result) in marks {
            machine.mark_trigger::<T>(i, result, false);
        }

        marks = Vec::default();

        for (i, trigger) in machine.get_triggers::<T>(true).into_iter().enumerate() {
            if let Ok(result) = trigger.trigger(entity, &param) {
                marks.push((i, result));
            }
        }

        for (i, result) in marks {
            machine.mark_trigger::<T>(i, result, true);
        }
    }
}

fn remove_done_markers(mut commands: Commands, dones: Query<Entity, With<Done>>) {
    for done in &dones {
        commands.entity(done).remove::<Done>();
    }
}
