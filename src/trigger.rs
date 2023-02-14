use std::{any::TypeId, marker::PhantomData};

use bevy::{
    ecs::system::{StaticSystemParam, SystemParam, SystemParamFetch},
    reflect::FromReflect,
};

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

/// Types that implement this may be used in [`StateMachine`]s to transition from one state
/// to another. [`TriggerPlugin`] must be added for each trigger. Since triggers
/// must implement [`FromReflect`] and [`Reflect`], enums may not be triggers. Look at an example
/// for implementing this trait, since it can be tricky without direction.
pub trait Trigger: 'static + FromReflect + Reflect + Send + Sync {
    /// System parameter provided to [`Trigger::trigger()`]. Must not access [`StateMachine`].
    type Param<'w, 's>: SystemParam;

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
    ) -> bool;

    /// Get the name of the type, for use in logging. You probably should not override this.
    fn base_type_name(&self) -> &str {
        self.type_name()
    }
}

/// Trigger that always transitions
#[derive(Debug, FromReflect, Reflect)]
pub struct AlwaysTrigger;

impl Trigger for AlwaysTrigger {
    type Param<'w, 's> = ();

    fn trigger(&self, _: Entity, _: &()) -> bool {
        true
    }
}

/// Trigger that negates the contained trigger. It is works for any trigger that is added
/// by [`TriggerPlugin`].
#[derive(Debug, Deref, DerefMut, FromReflect, Reflect)]
pub struct NotTrigger<T: Trigger>(pub T);

impl<T: Trigger> Trigger for NotTrigger<T> {
    type Param<'w, 's> = T::Param<'w, 's>;

    fn trigger(
        &self,
        entity: Entity,
        param: &<<<Self as Trigger>::Param<'_, '_> as SystemParam>::Fetch as SystemParamFetch<
            '_,
            '_,
        >>::Item,
    ) -> bool {
        let NotTrigger(trigger) = self;
        !trigger.trigger(entity, param)
    }

    fn base_type_name(&self) -> &str {
        (**self).base_type_name()
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
#[derive(Debug, FromReflect, Reflect)]
pub enum DoneTrigger {
    /// Success variant
    Success,
    /// Failure variant
    Failure,
}

impl Trigger for DoneTrigger {
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
        for (i, trigger) in machine.get_triggers::<T>().into_iter().enumerate() {
            if trigger.trigger(entity, &param) {
                machine.mark_trigger::<T>(i);
            }
        }
    }
}

fn remove_done_markers(mut commands: Commands, dones: Query<Entity, With<Done>>) {
    for done in &dones {
        commands.entity(done).remove::<Done>();
    }
}
