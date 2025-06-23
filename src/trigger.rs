//! Triggers are checked to determine whether the machine should transition to a new state. They can
//! be combined with the `not`, `and`, and `or` combinators. See [`EntityTrigger`].

#[cfg(feature = "leafwing_input")]
mod input;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{intern::Interned, schedule::ScheduleLabel};
use either::Either;
#[cfg(feature = "leafwing_input")]
pub use input::{
    action_data, axis_pair, axis_pair_length_bounds, axis_pair_max_length, axis_pair_min_length,
    axis_pair_rotation_bounds, axis_pair_unbounded, clamped_axis_pair,
    clamped_axis_pair_length_bounds, clamped_axis_pair_max_length, clamped_axis_pair_min_length,
    clamped_axis_pair_rotation_bounds, clamped_axis_pair_unbounded, clamped_value,
    clamped_value_max, clamped_value_min, clamped_value_unbounded, just_pressed, just_released,
    pressed, value, value_max, value_min, value_unbounded,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::{convert::Infallible, fmt::Debug};

use crate::{prelude::*, set::StateSet};

pub(crate) fn plug(schedule: Interned<dyn ScheduleLabel>) -> impl Fn(&mut App) {
    move |app| {
        app.configure_sets(
            schedule,
            StateSet::RemoveDoneMarkers.after(StateSet::Transition),
        )
        .add_systems(
            schedule,
            remove_done_markers.in_set(StateSet::RemoveDoneMarkers),
        );
    }
}

/// Wrapper for [`core::convert::Infallible`]. Use for [`EntityTrigger::Err`] if the trigger is
/// infallible.
#[derive(Debug, Deref, DerefMut, Clone, Copy, PartialEq, Eq)]
pub struct Never {
    never: Infallible,
}

/// Input requested by a trigger
pub trait TriggerIn: SystemInput {
    /// Convert an `Entity` to `Self`
    fn from_entity(entity: Entity) -> Self::Inner<'static>;
}

impl TriggerIn for () {
    fn from_entity(_: Entity) {}
}

impl TriggerIn for In<Entity> {
    fn from_entity(entity: Entity) -> Entity {
        entity
    }
}

/// Output returned from a trigger. Indicates whether the transition will occur, and may include
/// data given to `StateMachine::trans_builder`.
pub trait TriggerOut {
    /// Data given to `StateMachine::trans_builder` on a success
    type Ok;
    /// Data given to `StataMachine::trans_builder` if this trigger fails and is negated
    type Err;

    /// Convert `Self` to a `Result`
    fn into_result(self) -> Result<Self::Ok, Self::Err>;
}

impl TriggerOut for bool {
    type Ok = ();
    type Err = ();

    fn into_result(self) -> Result<(), ()> {
        if self {
            Ok(())
        } else {
            Err(())
        }
    }
}

impl<T> TriggerOut for Option<T> {
    type Ok = T;
    type Err = ();

    fn into_result(self) -> Result<T, ()> {
        self.ok_or(())
    }
}

impl<Ok, Err> TriggerOut for Result<Ok, Err> {
    type Ok = Ok;
    type Err = Err;

    fn into_result(self) -> Self {
        self
    }
}

/// Conversion trait to turn something into an [`EntityTrigger`].
///
/// Automatically implemented for types that implement [`EntityTrigger`] and certain types that
/// implement
/// [`IntoSystem`]. Types that implement [`IntoSystem`] don't automatically implement
/// [`EntityTrigger`],
/// so if you want to accept a trigger somewhere, you can accept a generic that implements this
/// trait instead. Otherwise, the caller will usually have to call `.into_trigger()` when providing
/// a type that implements [`IntoSystem`].
///
/// The `Marker` type param is necessary to implement this trait for systems, to prevent a system
/// from implementing the same instance of this trait multiple times, since a type may implement
/// multiple instances of [`IntoSystem`]. It doesn't matter what type `Marker` is set to.
pub trait IntoTrigger<Marker>: Sized {
    /// The [`EntityTrigger`] type that this is converted into
    type Trigger: EntityTrigger;

    /// Convert into an [`EntityTrigger`]
    fn into_trigger(self) -> Self::Trigger;

    /// Negates the trigger. Do not override.
    fn not(
        self,
    ) -> impl EntityTrigger<
        Out = Result<
            <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
            <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Ok,
        >,
    > {
        NotTrigger(self.into_trigger())
    }

    /// Combines these triggers by logical AND. Do not override.
    fn and<Marker2, T: IntoTrigger<Marker2>>(
        self,
        other: T,
    ) -> impl EntityTrigger<
        Out = Result<
            (
                <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Ok,
                <<T::Trigger as EntityTrigger>::Out as TriggerOut>::Ok,
            ),
            Either<
                <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
                <<T::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
            >,
        >,
    > {
        AndTrigger(self.into_trigger(), other.into_trigger())
    }

    /// Combines these triggers by logical AND, discarding the output of the first. Do not override.
    fn ignore_and<Marker2, T: IntoTrigger<Marker2>>(
        self,
        other: T,
    ) -> impl EntityTrigger<
        Out = Result<
            <<T::Trigger as EntityTrigger>::Out as TriggerOut>::Ok,
            Either<
                <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
                <<T::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
            >,
        >,
    > {
        IgnoreAndTrigger(self.into_trigger(), other.into_trigger())
    }

    /// Combines these triggers by logical OR. Do not override.
    fn or<Marker2, T: IntoTrigger<Marker2>>(
        self,
        other: T,
    ) -> impl EntityTrigger<
        Out = Result<
            Either<
                <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Ok,
                <<T::Trigger as EntityTrigger>::Out as TriggerOut>::Ok,
            >,
            (
                <<Self::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
                <<T::Trigger as EntityTrigger>::Out as TriggerOut>::Err,
            ),
        >,
    > {
        OrTrigger(self.into_trigger(), other.into_trigger())
    }
}

impl<I, O, Marker, T: IntoSystem<I, O, Marker>> IntoTrigger<(I, O, Marker)> for T
where
    I: TriggerIn,
    O: TriggerOut,
    T::System: ReadOnlySystem,
{
    type Trigger = SystemTrigger<T::System>;

    fn into_trigger(self) -> Self::Trigger {
        SystemTrigger(IntoSystem::into_system(self))
    }
}

/// Types that implement this may be used in [`StateMachine`]s to transition from one state to
/// another. Look at an example for implementing this trait, since it can be tricky.
pub trait EntityTrigger: 'static + Send + Sync {
    /// The trigger's output. See [`TriggerOut`].
    type Out: TriggerOut;

    /// Initializes/resets this trigger. Runs every time the state machine transitions.
    fn init(&mut self, world: &mut World);
    /// Checks whether the state machine should transition
    fn check(&mut self, entity: Entity, world: &World) -> Self::Out;
}

impl<T: EntityTrigger> IntoTrigger<()> for T {
    type Trigger = T;

    fn into_trigger(self) -> T {
        self
    }
}

impl<O: 'static + TriggerOut> EntityTrigger for Box<dyn EntityTrigger<Out = O>> {
    type Out = O;

    fn init(&mut self, world: &mut World) {
        (**self).init(world);
    }

    fn check(&mut self, entity: Entity, world: &World) -> Self::Out {
        (**self).check(entity, world)
    }
}

/// The trigger form of a system. See [`IntoSystem`].
pub struct SystemTrigger<T: ReadOnlySystem>(T);

impl<T: ReadOnlySystem> EntityTrigger for SystemTrigger<T>
where
    T::In: TriggerIn,
    T::Out: TriggerOut,
{
    type Out = T::Out;

    fn init(&mut self, world: &mut World) {
        let Self(t) = self;
        t.initialize(world);
    }

    fn check(&mut self, entity: Entity, world: &World) -> Self::Out {
        let Self(t) = self;
        t.run_readonly(T::In::from_entity(entity), world)
    }
}

/// Trigger that always transitions
pub fn always() -> bool {
    true
}

/// Negates the given trigger
#[derive(Debug)]
pub struct NotTrigger<T: EntityTrigger>(pub T);

impl<T: EntityTrigger> EntityTrigger for NotTrigger<T> {
    type Out = Result<<T::Out as TriggerOut>::Err, <T::Out as TriggerOut>::Ok>;

    fn init(&mut self, world: &mut World) {
        let Self(t) = self;
        t.init(world);
    }

    fn check(&mut self, entity: Entity, world: &World) -> Self::Out {
        let Self(t) = self;
        match t.check(entity, world).into_result() {
            Ok(ok) => Err(ok),
            Err(err) => Ok(err),
        }
    }
}

/// Combines two triggers by logical AND
#[derive(Debug)]
pub struct AndTrigger<T: EntityTrigger, U: EntityTrigger>(pub T, pub U);

impl<T: EntityTrigger, U: EntityTrigger> EntityTrigger for AndTrigger<T, U> {
    type Out = Result<
        (<T::Out as TriggerOut>::Ok, <U::Out as TriggerOut>::Ok),
        Either<<T::Out as TriggerOut>::Err, <U::Out as TriggerOut>::Err>,
    >;

    fn init(&mut self, world: &mut World) {
        let Self(t, u) = self;

        t.init(world);
        u.init(world);
    }

    fn check(&mut self, entity: Entity, world: &World) -> Self::Out {
        let Self(t, u) = self;

        Ok((
            t.check(entity, world).into_result().map_err(Either::Left)?,
            u.check(entity, world)
                .into_result()
                .map_err(Either::Right)?,
        ))
    }
}

/// Combines two triggers by logical AND, discarding the output of the first
#[derive(Debug)]
pub struct IgnoreAndTrigger<T: EntityTrigger, U: EntityTrigger>(pub T, pub U);

impl<T: EntityTrigger, U: EntityTrigger> EntityTrigger for IgnoreAndTrigger<T, U> {
    type Out = Result<
        <U::Out as TriggerOut>::Ok,
        Either<<T::Out as TriggerOut>::Err, <U::Out as TriggerOut>::Err>,
    >;

    fn init(&mut self, world: &mut World) {
        let Self(t, u) = self;

        t.init(world);
        u.init(world);
    }

    fn check(&mut self, entity: Entity, world: &World) -> Self::Out {
        let Self(t, u) = self;

        t.check(entity, world).into_result().map_err(Either::Left)?;
        u.check(entity, world).into_result().map_err(Either::Right)
    }
}

/// Combines two triggers by logical OR
#[derive(Debug)]
pub struct OrTrigger<T: EntityTrigger, U: EntityTrigger>(pub T, pub U);

impl<T: EntityTrigger, U: EntityTrigger> EntityTrigger for OrTrigger<T, U> {
    type Out = Result<
        Either<<T::Out as TriggerOut>::Ok, <U::Out as TriggerOut>::Ok>,
        (<T::Out as TriggerOut>::Err, <U::Out as TriggerOut>::Err),
    >;

    fn init(&mut self, world: &mut World) {
        let Self(t, u) = self;

        t.init(world);
        u.init(world);
    }

    fn check(&mut self, entity: Entity, world: &World) -> Self::Out {
        let Self(t, u) = self;

        match t.check(entity, world).into_result() {
            Ok(ok) => Ok(Either::Left(ok)),
            Err(err_1) => match u.check(entity, world).into_result() {
                Ok(ok) => Ok(Either::Right(ok)),
                Err(err_2) => Err((err_1, err_2)),
            },
        }
    }
}

/// Marker component that represents that the current state has completed. Removed from every entity
/// each frame after checking triggers. To be used with [`done`].
#[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[component(storage = "SparseSet")]
pub enum Done {
    /// Success variant
    Success,
    /// Failure variant
    Failure,
}

/// Trigger that transitions if the entity has the [`Done`] component. Provide `Some(Done::Variant)`
/// to transition upon that particular variant, or `None` to transition upon either.
pub fn done(expected: Option<Done>) -> impl EntityTrigger<Out = bool> {
    (move |In(entity): In<Entity>, dones: Query<&Done>| {
        dones
            .get(entity)
            .map(|&done| expected.is_none() || Some(done) == expected)
            .unwrap_or(false)
    })
    .into_trigger()
}

/// Trigger that transitions when it receives the associated event
pub fn on_event<T: Clone + Event>(
    mut has_run: Local<bool>,
    mut reader: EventReader<T>,
) -> Option<T> {
    if !*has_run {
        reader.read().last();
        *has_run = true;
        return None;
    }

    reader.read().last().cloned()
}

pub(crate) fn remove_done_markers(mut commands: Commands, dones: Query<Entity, With<Done>>) {
    for done in &dones {
        commands.entity(done).remove::<Done>();
    }
}
