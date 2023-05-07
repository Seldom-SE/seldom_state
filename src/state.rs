use std::{
    any::TypeId,
    fmt::{self, Debug, Formatter},
};

use bevy::ecs::system::{Command, EntityCommands};

use crate::prelude::*;

/// A state that an entity may be in.
///
/// If you are concerned with performance, consider having your states use sparse set storage,
/// since they may be added to and removed from entities.
pub trait MachineState: 'static + Clone + Send + Sync {
    fn from_entity(entity: Entity, world: &World) -> &Self;
    fn remove(entity: &mut EntityCommands);
}

impl<T: Clone + Component> MachineState for T {
    fn from_entity(entity: Entity, world: &World) -> &Self {
        world.entity(entity).get().unwrap()
    }

    fn remove(entity: &mut EntityCommands) {
        entity.remove::<Self>();
    }
}

/// State that represents any state. Transitions from [`AnyState`] may transition
/// from any other state.
#[derive(Clone, Debug)]
pub struct AnyState(());

impl MachineState for AnyState {
    fn from_entity(_: Entity, _: &World) -> &Self {
        &AnyState(())
    }

    fn remove(_: &mut EntityCommands) {}
}

pub(crate) trait Insert: Send {
    fn insert(self: Box<Self>, entity: &mut EntityCommands) -> TypeId;
}

impl<S: Component> Insert for S {
    fn insert(self: Box<Self>, entity: &mut EntityCommands) -> TypeId {
        entity.insert(*self);
        TypeId::of::<AnyState>()
    }
}

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
        commands.add(self.clone())
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
        app.add_system(transition);

        let machine = StateMachine::default()
            .trans::<StateOne>(AlwaysTrigger, StateTwo)
            .on_exit::<StateOne>(|commands| commands.commands().insert_resource(SomeResource))
            .on_enter::<StateTwo>(|commands| commands.commands().insert_resource(AnotherResource));

        let entity = app.world.spawn((machine, StateOne)).id();
        assert!(app.world.get::<StateOne>(entity).is_some());

        app.update();
        assert!(app.world.get::<StateTwo>(entity).is_some());
        assert!(
            app.world.contains_resource::<SomeResource>(),
            "exit state triggers should run"
        );
        assert!(
            app.world.contains_resource::<AnotherResource>(),
            "exit state triggers should run"
        );
    }
}
