#[cfg(feature = "leafwing_input")]
mod input;

#[cfg(feature = "leafwing_input")]
pub use input::{
    ActionDataTrigger, AxisPairTrigger, ClampedAxisPairTrigger, ClampedValueTrigger,
    JustPressedTrigger, JustReleasedTrigger, PressedTrigger, ReleasedTrigger, ValueTrigger,
};

use std::{convert::Infallible, fmt::Debug};

use bevy::ecs::system::{ReadOnlySystemParam, SystemParam};

use crate::prelude::*;

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
pub trait Trigger: 'static + Reflect + Send + Sync {
    /// System parameter provided to [`Trigger::trigger`]. Must not access [`StateMachine`].
    type Param<'w, 's>: ReadOnlySystemParam;
    /// When the trigger occurs, this data is returned from `trigger`, and passed
    /// to every transition builder on this trigger. If there's no relevant information to pass,
    /// just use `()`. If there's also no relevant information to pass to [`Trigger::Err`],
    /// implement [`BoolTrigger`] instead.
    type Ok: Reflect;
    /// When the trigger does not occur, this data is returned from `trigger`. In this case,
    /// [`NotTrigger<Self>`] passes it to every transition builder on this trigger.
    /// If there's no relevant information to pass, implement [`OptionTrigger`] instead.
    /// If this trigger is infallible, use [`Never`].
    type Err: Reflect;

    /// Called for every entity that may transition to a state on this trigger. Return `Ok`
    /// if it should transition, and `Err` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: &<<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<Self::Ok, Self::Err>;

    /// Get the name of the type, for use in logging. You probably should not override this.
    fn base_type_name(&self) -> &str {
        self.type_name()
    }
}

/// Automatically implements [`Trigger`]. Implement this instead of there is no relevant
/// information to pass for [`Trigger::Err`].
pub trait OptionTrigger: 'static + Reflect + Send + Sync {
    /// System parameter provided to [`OptionTrigger::trigger`]. Must not access [`StateMachine`].
    type Param<'w, 's>: ReadOnlySystemParam;
    /// When the trigger occurs, this data is returned from `trigger`, and passed
    /// to every transition builder on this trigger. If there's no relevant information to pass,
    /// implement [`BoolTrigger`] instead.
    type Some: Reflect;

    /// Called for every entity that may transition to a state on this trigger. Return `Some`
    /// if it should transition, and `None` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: &<<Self as OptionTrigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Some>;
}

impl<T: OptionTrigger> Trigger for T {
    type Param<'w, 's> = <Self as OptionTrigger>::Param<'w, 's>;
    type Ok = <Self as OptionTrigger>::Some;
    type Err = ();

    fn trigger(
        &self,
        entity: Entity,
        param: &<<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<Self::Ok, ()> {
        OptionTrigger::trigger(self, entity, param).ok_or(())
    }
}

/// Automatically implements [`Trigger`]. Implement this instead of there is no relevant
/// information to pass for [`Trigger::Ok`] and [`Trigger::Err`].
pub trait BoolTrigger: 'static + Reflect + Send + Sync {
    /// System parameter provided to [`BoolTrigger::trigger`]. Must not access [`StateMachine`].
    type Param<'w, 's>: ReadOnlySystemParam;

    /// Called for every entity that may transition to a state on this trigger. Return `true`
    /// if it should transition, and `false` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: &<<Self as BoolTrigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> bool;
}

impl<T: BoolTrigger> OptionTrigger for T {
    type Param<'w, 's> = <Self as BoolTrigger>::Param<'w, 's>;
    type Some = ();

    fn trigger(
        &self,
        entity: Entity,
        param: &<<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Option<()> {
        BoolTrigger::trigger(self, entity, param).then_some(())
    }
}

/// Trigger that always transitions
#[derive(Debug, Reflect)]
pub struct AlwaysTrigger;

impl Trigger for AlwaysTrigger {
    type Param<'w, 's> = ();
    type Ok = ();
    type Err = Never;

    fn trigger(&self, _: Entity, _: &()) -> Result<(), Never> {
        Ok(())
    }
}

/// Trigger that negates the contained trigger.
#[derive(Debug, Deref, DerefMut, Reflect)]
pub struct NotTrigger<T: Trigger>(pub T);

impl<T: Trigger> Trigger for NotTrigger<T> {
    type Param<'w, 's> = T::Param<'w, 's>;
    type Ok = T::Err;
    type Err = T::Ok;

    fn trigger(
        &self,
        entity: Entity,
        param: &<<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<T::Err, T::Ok> {
        let Self(trigger) = self;
        match trigger.trigger(entity, param) {
            Ok(ok) => Err(ok),
            Err(err) => Ok(err),
        }
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
#[derive(Debug, Reflect)]
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

pub(crate) fn remove_done_markers(mut commands: Commands, dones: Query<Entity, With<Done>>) {
    for done in &dones {
        commands.entity(done).remove::<Done>();
    }
}
