//! Triggers are checked to determine whether the machine should transition to a new state. They can
//! be combined with the `not`, `and`, and `or` combinators. See [`Trigger`].

#[cfg(feature = "leafwing_input")]
mod input;

use either::Either;
#[cfg(feature = "leafwing_input")]
pub use input::{
    ActionDataTrigger, AxisPairTrigger, ClampedAxisPairTrigger, ClampedValueTrigger,
    JustPressedTrigger, JustReleasedTrigger, PressedTrigger, ReleasedTrigger, ValueTrigger,
};

use std::{any::type_name, convert::Infallible, fmt::Debug, marker::PhantomData};

use bevy::ecs::system::{ReadOnlySystemParam, SystemParam};

use crate::{prelude::*, set::StateSet};

pub(crate) fn trigger_plugin(app: &mut App) {
    app.configure_set(
        PostUpdate,
        StateSet::RemoveDoneMarkers.after(StateSet::Transition),
    )
    .add_systems(
        PostUpdate,
        remove_done_markers.in_set(StateSet::RemoveDoneMarkers),
    );
}

/// Wrapper for [`core::convert::Infallible`]. Use for [`Trigger::Err`] if the trigger is
/// infallible.
#[derive(Debug, Deref, DerefMut, Clone, Copy, PartialEq, Eq)]
pub struct Never {
    never: Infallible,
}

/// Types that implement this may be used in [`StateMachine`]s to transition from one state to
/// another. Look at an example for implementing this trait, since it can be tricky.
pub trait Trigger: 'static + Send + Sized + Sync {
    /// System parameter provided to [`Trigger::trigger`]
    type Param<'w, 's>: ReadOnlySystemParam;
    /// When the trigger occurs, this data is returned from `trigger`, and passed to every
    /// transition builder on this trigger. If there's no relevant information to pass, just use
    /// `()`. If there's also no relevant information to pass to [`Trigger::Err`], implement
    /// [`BoolTrigger`] instead.
    type Ok;
    /// When the trigger does not occur, this data is returned from `trigger`. In this case,
    /// [`NotTrigger<Self>`] passes it to every transition builder on this trigger. If there's no
    /// relevant information to pass, implement [`OptionTrigger`] instead. If this trigger is
    /// infallible, use [`Never`].
    type Err;

    /// Called for every entity that may transition to a state on this trigger. Return `Ok` if it
    /// should transition, and `Err` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: <<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<Self::Ok, Self::Err>;

    /// Gets the name of the type, for use in logging
    fn base_type_name(&self) -> &str {
        type_name::<Self>()
    }

    /// Negates the trigger
    fn not(self) -> NotTrigger<Self> {
        NotTrigger(self)
    }

    /// Combines these triggers by logical AND
    fn and<T: Trigger>(self, other: T) -> AndTrigger<Self, T> {
        AndTrigger(self, other)
    }

    /// Combines these triggers by logical OR
    fn or<T: Trigger>(self, other: T) -> OrTrigger<Self, T> {
        OrTrigger(self, other)
    }
}

/// Automatically implements [`Trigger`]. Implement this instead if there is no relevant information
/// to pass for [`Trigger::Err`].
pub trait OptionTrigger: 'static + Send + Sync {
    /// System parameter provided to [`OptionTrigger::trigger`]
    type Param<'w, 's>: ReadOnlySystemParam;
    /// When the trigger occurs, this data is returned from `trigger`, and passed to every
    /// transition builder on this trigger. If there's no relevant information to pass, implement
    /// [`BoolTrigger`] instead.
    type Some;

    /// Called for every entity that may transition to a state on this trigger. Return `Some` if it
    /// should transition, and `None` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: <<Self as OptionTrigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Some>;
}

impl<T: OptionTrigger> Trigger for T {
    type Param<'w, 's> = <Self as OptionTrigger>::Param<'w, 's>;
    type Ok = <Self as OptionTrigger>::Some;
    type Err = ();

    fn trigger(
        &self,
        entity: Entity,
        param: <<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<Self::Ok, ()> {
        OptionTrigger::trigger(self, entity, param).ok_or(())
    }
}

/// Automatically implements [`Trigger`]. Implement this instead if there is no relevant information
/// to pass for [`Trigger::Ok`] and [`Trigger::Err`].
pub trait BoolTrigger: 'static + Send + Sync {
    /// System parameter provided to [`BoolTrigger::trigger`]
    type Param<'w, 's>: ReadOnlySystemParam;

    /// Called for every entity that may transition to a state on this trigger. Return `true` if it
    /// should transition, and `false` if it should not. In most cases, you may use
    /// `&Self::Param<'_, '_>` as `param`'s type.
    fn trigger(
        &self,
        entity: Entity,
        param: <<Self as BoolTrigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> bool;
}

impl<T: BoolTrigger> OptionTrigger for T {
    type Param<'w, 's> = <Self as BoolTrigger>::Param<'w, 's>;
    type Some = ();

    fn trigger(
        &self,
        entity: Entity,
        param: <<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Option<()> {
        BoolTrigger::trigger(self, entity, param).then_some(())
    }
}

/// Trigger that always transitions
#[derive(Debug, Clone, Copy)]
pub struct AlwaysTrigger;

impl Trigger for AlwaysTrigger {
    type Param<'w, 's> = ();
    type Ok = ();
    type Err = Never;

    fn trigger(&self, _: Entity, _: ()) -> Result<(), Never> {
        Ok(())
    }
}

/// Trigger that negates the contained trigger
#[derive(Debug, Deref, DerefMut)]
pub struct NotTrigger<T: Trigger>(pub T);

impl<T: Trigger> Trigger for NotTrigger<T> {
    type Param<'w, 's> = T::Param<'w, 's>;
    type Ok = T::Err;
    type Err = T::Ok;

    fn trigger(
        &self,
        entity: Entity,
        param: <<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<T::Err, T::Ok> {
        let Self(trigger) = self;
        match trigger.trigger(entity, param) {
            Ok(ok) => Err(ok),
            Err(err) => Ok(err),
        }
    }
}

impl<T: Trigger + Clone> Clone for NotTrigger<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Trigger + Copy> Copy for NotTrigger<T> {}

/// Trigger that combines two triggers by logical AND
#[derive(Debug)]
pub struct AndTrigger<T: Trigger, U: Trigger>(pub T, pub U);

impl<T: Trigger, U: Trigger> Trigger for AndTrigger<T, U> {
    type Param<'w, 's> = (T::Param<'w, 's>, U::Param<'w, 's>);
    type Ok = (T::Ok, U::Ok);
    type Err = Either<T::Err, U::Err>;

    fn trigger(
        &self,
        entity: Entity,
        (param1, param2): <<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<(T::Ok, U::Ok), Either<T::Err, U::Err>> {
        let Self(trigger1, trigger2) = self;
        Ok((
            trigger1.trigger(entity, param1).map_err(Either::Left)?,
            trigger2.trigger(entity, param2).map_err(Either::Right)?,
        ))
    }
}

impl<T: Trigger + Clone, U: Trigger + Clone> Clone for AndTrigger<T, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}
impl<T: Trigger + Copy, U: Trigger + Copy> Copy for AndTrigger<T, U> {}

/// Trigger that combines two triggers by logical OR
#[derive(Debug)]
pub struct OrTrigger<T: Trigger, U: Trigger>(pub T, pub U);

impl<T: Trigger, U: Trigger> Trigger for OrTrigger<T, U> {
    type Param<'w, 's> = (T::Param<'w, 's>, U::Param<'w, 's>);
    type Ok = Either<T::Ok, U::Ok>;
    type Err = (T::Err, U::Err);

    fn trigger(
        &self,
        entity: Entity,
        (param1, param2): <<Self as Trigger>::Param<'_, '_> as SystemParam>::Item<'_, '_>,
    ) -> Result<Either<T::Ok, U::Ok>, (T::Err, U::Err)> {
        let Self(trigger1, trigger2) = self;
        match trigger1.trigger(entity, param1) {
            Ok(ok) => Ok(Either::Left(ok)),
            Err(err1) => match trigger2.trigger(entity, param2) {
                Ok(ok) => Ok(Either::Right(ok)),
                Err(err2) => Err((err1, err2)),
            },
        }
    }
}

impl<T: Trigger + Clone, U: Trigger + Clone> Clone for OrTrigger<T, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}
impl<T: Trigger + Copy, U: Trigger + Copy> Copy for OrTrigger<T, U> {}

/// Marker component that represents that the current state has completed. Removed from every entity
/// each frame after checking triggers. To be used with [`DoneTrigger`].
#[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
#[component(storage = "SparseSet")]
pub enum Done {
    /// Success variant
    Success,
    /// Failure variant
    Failure,
}

/// Trigger that transitions if the entity has the [`Done`] component with the associated variant
#[derive(Debug, Clone, Copy)]
pub enum DoneTrigger {
    /// Success variant
    Success,
    /// Failure variant
    Failure,
}

impl BoolTrigger for DoneTrigger {
    type Param<'w, 's> = Query<'w, 's, &'static Done>;

    fn trigger(&self, entity: Entity, param: Self::Param<'_, '_>) -> bool {
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

/// Trigger that transitions when it receives the associated event
#[derive(Debug, Default, Clone, Copy)]
pub struct EventTrigger<T: Clone + Event>(PhantomData<T>);

impl<T: Clone + Event> OptionTrigger for EventTrigger<T> {
    type Param<'w, 's> = EventReader<'w, 's, T>;
    type Some = T;

    fn trigger(
        &self,
        _: Entity,
        mut events: Self::Param<'_, '_>,
    ) -> Option<<Self as OptionTrigger>::Some> {
        events.iter().next().cloned()
    }
}

pub(crate) fn remove_done_markers(mut commands: Commands, dones: Query<Entity, With<Done>>) {
    for done in &dones {
        commands.entity(done).remove::<Done>();
    }
}
