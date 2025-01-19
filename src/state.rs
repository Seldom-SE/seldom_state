use std::{
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
};

use bevy::ecs::{system::EntityCommands, world::Command};

use crate::prelude::*;

use self::sealed::EntityStateSealed;

mod sealed {
    use std::{any::TypeId, marker::PhantomData};

    use bevy::utils::all_tuples;

    use crate::prelude::*;

    pub trait EntityStateSealed: Sized {
        fn matches(state: TypeId) -> bool;
        fn remove(entity: Entity, world: &mut World, curr: TypeId) -> Self;
    }

    impl<T: Clone + Component> EntityStateSealed for T {
        fn matches(state: TypeId) -> bool {
            state == TypeId::of::<T>()
        }

        fn remove(entity: Entity, world: &mut World, _: TypeId) -> Self {
            world.entity_mut(entity).take::<Self>().unwrap()
        }
    }

    macro_rules! impl_entity_state_sealed {
        ($($T:ident),*) => {
            #[allow(unused)]
            impl<$($T: EntityStateSealed),*> EntityStateSealed for OneOfState<($($T,)*)> {
                fn matches(state: TypeId) -> bool {
                    $($T::matches(state) ||)* false
                }

                fn remove(entity: Entity, world: &mut World, curr: TypeId) -> Self {
                    $(
                        if $T::matches(curr) {
                            $T::remove(entity, world, curr);
                        }
                    )*

                    Self(PhantomData)
                }
            }
        };
    }

    all_tuples!(impl_entity_state_sealed, 0, 15, T);

    impl EntityStateSealed for AnyState {
        fn matches(_: TypeId) -> bool {
            true
        }

        fn remove(entity: Entity, world: &mut World, curr: TypeId) -> Self {
            let curr = world.components().get_id(curr).unwrap();
            world.entity_mut(entity).remove_by_id(curr);
            AnyState(())
        }
    }
}

/// A state that an entity may be in.
///
/// If you are concerned with performance, consider having your states use sparse set storage if
/// transitions are very frequent.
pub trait EntityState: 'static + Clone + Send + Sync + EntityStateSealed {}

impl<T: Clone + Component> EntityState for T {}

/// State that represents any of the states given in a tuple (ex
/// `OneOfState<(Idle, Jump, Attack)>`).
#[derive(Clone, Debug)]
pub struct OneOfState<T>(pub(crate) PhantomData<T>);

impl<T: 'static + Clone + Send + Sync> EntityState for OneOfState<T> where Self: EntityStateSealed {}

/// State that represents any state. Transitions from [`AnyState`] may transition from any other
/// state.
#[derive(Clone, Debug)]
pub struct AnyState(pub(crate) ());

impl EntityState for AnyState {}

#[derive(Debug)]
pub(crate) enum OnEvent {
    Entity(Box<dyn EntityEvent>),
    Command(Box<dyn CommandEvent>),
}

impl OnEvent {
    pub(crate) fn trigger(&self, entity: Entity, commands: &mut Commands) {
        match self {
            OnEvent::Entity(event) => event.trigger(&mut commands.entity(entity)),
            OnEvent::Command(event) => event.trigger(commands),
        }
    }
}

pub(crate) trait EntityEvent: Send + Sync {
    fn trigger(&self, entity: &mut EntityCommands);
}

impl Debug for dyn EntityEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Fn(&mut EntityCommands)")
    }
}

impl<F: Fn(&mut EntityCommands) + Send + Sync> EntityEvent for F {
    fn trigger(&self, entity: &mut EntityCommands) {
        self(entity)
    }
}

pub(crate) trait CommandEvent: Command + Sync {
    fn trigger(&self, commands: &mut Commands);
}

impl Debug for dyn CommandEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Command")
    }
}

impl<C: Clone + Command + Sync> CommandEvent for C {
    fn trigger(&self, commands: &mut Commands) {
        commands.queue(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::machine::transition;

    use super::*;

    #[derive(Component, Clone)]
    struct StateOne;
    #[derive(Component, Clone)]
    struct StateTwo;

    #[derive(Resource, Clone)]
    struct SomeResource;
    #[derive(Resource, Clone)]
    struct AnotherResource;

    #[test]
    fn test_triggers() {
        let mut app = App::new();
        app.add_systems(Update, transition);

        let machine = StateMachine::default()
            .trans::<StateOne, _>(always, StateTwo)
            .on_exit::<StateOne>(|commands| commands.commands().insert_resource(SomeResource))
            .on_enter::<StateTwo>(|commands| commands.commands().insert_resource(AnotherResource));

        let entity = app.world_mut().spawn((machine, StateOne)).id();
        assert!(app.world().get::<StateOne>(entity).is_some());

        app.update();
        assert!(app.world().get::<StateTwo>(entity).is_some());
        assert!(
            app.world().contains_resource::<SomeResource>(),
            "exit state triggers should run"
        );
        assert!(
            app.world().contains_resource::<AnotherResource>(),
            "exit state triggers should run"
        );
    }
}
