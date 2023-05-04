use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::ecs::system::SystemState;
use bevy::ecs::system::{Command, EntityCommands};

use crate::{prelude::*, state::OnEvent};

/// This represents something that can be used to perform a transition. We have
/// a trait for this so we can box it up.
trait Transition: Debug + Send + Sync + 'static {
    /// This is guaranteed to be called before all calls to `run`.
    fn initialize(&mut self, world: &mut World);
    /// Check whether the transition should be taken. `entity` is the entity
    /// that contains the state machine. If you want to take the transition,
    /// use `commands.insert` and return the TypeId you inserted.
    fn run(&mut self, world: &World, commands: &mut EntityCommands) -> Option<TypeId>;
}

/// This represents an edge in the state machine. The type parameters represent
/// the [`Trigger`] that causes this transition to be taken, the function that
/// takes the trigger's output and builds the next state, and the next state
/// itself.
pub struct TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Build: Fn(Trig::Ok) -> Option<Next>,
{
    pub trigger: Trig,
    pub builder: Build,
    // In order to actually run this, we need to carry around a SystemState. We can't initialize
    // that until we have a World, so this starts out empty.
    system_state: Option<SystemState<Trig::Param<'static, 'static>>>,
    phantom: PhantomData<Prev>,
}

impl<Trig, Prev, Build, Next> Debug for TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Build: Fn(Trig::Ok) -> Option<Next> + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionImpl")
            .field("trigger", &self.trigger.type_id())
            .field("builder", &self.builder.type_id())
            .field("system_state", &self.system_state.type_id())
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<Trig, Prev, Build, Next> TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Prev: Component,
    Build: Fn(Trig::Ok) -> Option<Next>,
    Next: Component,
{
    pub fn new(trigger: Trig, builder: Build) -> Self {
        Self {
            trigger,
            builder,
            system_state: None,
            phantom: PhantomData,
        }
    }
}

impl<Trig, Prev, Build, Next> Transition for TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Prev: Component,
    Build: Fn(Trig::Ok) -> Option<Next> + Send + Sync + 'static,
    Next: Component,
{
    fn initialize(&mut self, world: &mut World) {
        self.system_state = Some(SystemState::new(world));
    }

    fn run(&mut self, world: &World, entity: &mut EntityCommands) -> Option<TypeId> {
        let state = self.system_state.as_mut().unwrap();
        if let Ok(res) = self.trigger.trigger(entity.id(), &state.get(world)) {
            if let Some(next) = (self.builder)(res) {
                entity.remove::<Prev>();
                entity.insert(next);
                return Some(TypeId::of::<Next>());
            }
        }
        None
    }
}

#[derive(Debug)]
struct StateMetadata {
    name: String,
    transitions: Vec<Box<dyn Transition>>,
    on_enter: Vec<OnEvent>,
    on_exit: Vec<OnEvent>,
}

impl StateMetadata {
    fn new<S: Bundle>() -> Self {
        Self {
            name: type_name::<S>().to_owned(),
            transitions: default(),
            on_enter: default(),
            on_exit: vec![OnEvent::Entity(Box::new(|entity: &mut EntityCommands| {
                entity.remove::<S>();
            }))],
        }
    }

    // We temporarily replace the StateMetadata with this during machine execution.
    fn dummy() -> Self {
        Self {
            name: "StateMetadata is being borrowed".to_owned(),
            transitions: default(),
            on_enter: default(),
            on_exit: default(),
        }
    }
}

/// State machine component. Entities with this component will have bundles (the states)
/// added and removed based on the transitions that you add. Build one
/// with `StateMachine::new`, `StateMachine::trans`, and other methods.
#[derive(Component, Debug)]
pub struct StateMachine {
    current: TypeId,
    states: HashMap<TypeId, StateMetadata>,
    log_transitions: bool,
}

impl StateMachine {
    /// Creates a state machine with an initial state and no transitions. Add more
    /// with [`StateMachine::trans`].
    pub fn new(initial: impl MachineState) -> Self {
        Self {
            current: initial.type_id(),
            states: HashMap::new(),
            log_transitions: false,
        }
    }

    /// Adds a transition to the state machine. When the entity is in the state
    /// given as a type parameter, and the given trigger occurs, it will transition
    /// to the state given as a function parameter.
    pub fn trans<S: MachineState>(self, trigger: impl Trigger, state: impl MachineState) -> Self {
        self.trans_builder::<S, _, _>(trigger, move |_| Some(state.clone()))
    }

    /// Adds a transition builder to the state machine. When the entity is in `P` state, and `T`
    /// occurs, the given builder will be run on `T`'s `Result` type. If the builder returns
    /// `Some(N)`, the machine will transition to that `N` state.
    pub fn trans_builder<P: MachineState, T: Trigger, N: MachineState>(
        mut self,
        trigger: T,
        builder: impl 'static + Clone + Fn(T::Ok) -> Option<N> + Send + Sync,
    ) -> Self {
        let transition: TransitionImpl<T, P, _, N> = TransitionImpl::new(trigger, builder);
        self.states
            .entry(TypeId::of::<P>())
            .or_insert(StateMetadata::new::<P>())
            .transitions
            .push(Box::new(transition) as Box<dyn Transition>);
        self
    }

    /// Adds an on-enter event to the state machine. Whenever the state machine transitions
    /// into the given state, it will run the event.
    pub fn on_enter<S: MachineState>(
        mut self,
        on_enter: impl 'static + Fn(&mut EntityCommands) + Send + Sync,
    ) -> Self {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
            .on_enter
            .push(OnEvent::Entity(Box::new(on_enter)));

        self
    }

    /// Adds an on-exit event to the state machine. Whenever the state machine transitions
    /// from the given state, it will run the command.
    pub fn on_exit<S: MachineState>(
        mut self,
        on_exit: impl 'static + Fn(&mut EntityCommands) + Send + Sync,
    ) -> Self {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
            .on_exit
            .push(OnEvent::Entity(Box::new(on_exit)));

        self
    }

    /// Adds an on-enter command to the state machine. Whenever the state machine transitions
    /// into the given state, it will run the command.
    pub fn command_on_enter<S: MachineState>(
        mut self,
        command: impl Clone + Command + Sync,
    ) -> Self {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
            .on_enter
            .push(OnEvent::Command(Box::new(command)));

        self
    }

    /// Adds an on-exit command to the state machine. Whenever the state machine transitions
    /// from the given state, it will run the command.
    pub fn command_on_exit<S: MachineState>(
        mut self,
        command: impl Clone + Command + Sync,
    ) -> Self {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
            .on_exit
            .push(OnEvent::Command(Box::new(command)));

        self
    }

    /// Sets whether transitions are logged to the console
    pub fn set_trans_logging(mut self, log_transitions: bool) -> Self {
        self.log_transitions = log_transitions;
        self
    }

    /// Updates the machine's state. Runs on_enter/exit triggers, but does
    /// *not* actually insert or remove components (including the old
    /// state). `entity` must point to the entity this state machine is on.
    fn update_state(&mut self, next_state: TypeId, entity: Entity, commands: &mut Commands) {
        let from = &self.states[&self.current];
        let to = &self.states[&next_state];
        for event in from.on_exit.iter() {
            event.trigger(entity, commands);
        }
        for event in to.on_enter.iter() {
            event.trigger(entity, commands);
        }
        if self.log_transitions {
            info!("{entity:?} transitioned from {} to {}", from.name, to.name);
        }
        self.current = next_state;
    }
}

/// Bookkeeping data used when applying the transition system.
struct Transitions {
    /// The entity we're currently on.
    entity: Entity,
    /// The state this transition leaves. We need this because we construct this
    /// by `take`ing the edges out of the state machine.
    old_state: TypeId,
    new_state: Option<TypeId>,
    metadata: StateMetadata,
}

impl Transitions {
    /// Construct this by temporarily pulling out the transitions from the state
    /// machine.
    fn take(machine: &mut StateMachine, entity: Entity, state: TypeId) -> Self {
        let metadata = machine.states.get_mut(&state).unwrap();
        Transitions {
            entity,
            old_state: state,
            new_state: None,
            // just gonna borrow this real quick
            metadata: std::mem::replace(metadata, StateMetadata::dummy()),
        }
    }

    /// Update the machine's state if the state was transitioned to, then put
    /// back the state.
    fn put_back(mut self, machine: &mut StateMachine) {
        let transitions = machine.states.get_mut(&self.old_state).unwrap();
        if let Some(new_state) = self.new_state {
            machine.current = new_state;
        }
        std::mem::swap(transitions, &mut self.metadata);
    }

    /// Initialize all transitions. Must be executed before `run`. This is
    /// separate because `run` is parallelizable (takes a `&World`) but this
    /// isn't (takes a `&mut World`).
    fn initialize(&mut self, world: &mut World) {
        for transition in self.metadata.transitions.iter_mut() {
            transition.initialize(world);
        }
    }

    /// Runs all transitions until one is actually taken. If one was taken,
    /// stores the new state in this so we can update the machine in `put_back`.
    fn run(&mut self, world: &World, commands: &mut Commands) {
        self.new_state = self
            .metadata
            .transitions
            .iter_mut()
            .find_map(|transition| transition.run(world, &mut commands.entity(self.entity)));
    }
}

/// Runs all transitions on all entities.
pub(crate) fn transition_system(world: &mut World, system_state: &mut SystemState<Commands>) {
    // pull the transitions out of the world
    let mut query = world.query::<(Entity, &mut StateMachine)>();
    let mut to_invoke: Vec<Transitions> = query
        .iter_mut(world)
        .map(|(entity, mut machine)| {
            let current = machine.current;
            Transitions::take(machine.as_mut(), entity, current)
        })
        .collect();

    for transition in to_invoke.iter_mut() {
        transition.initialize(world);
    }

    let mut commands = system_state.get(world);
    {
        // reborrow as immutable just to show that we can
        let world: &World = world;

        // TODO: do this in parallel.
        for transitions in to_invoke.iter_mut() {
            transitions.run(world, &mut commands);
        }
    }

    for transitions in to_invoke {
        let (_, mut machine) = query.get_mut(world, transitions.entity).unwrap();
        let next_state = transitions.new_state;
        transitions.put_back(machine.as_mut());
        if let Some(next_state) = next_state {
            machine.current = next_state;
        }
    }
    system_state.apply(world);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Just some test states to transition between.

    #[derive(Component, Clone, Reflect)]
    struct StateOne;
    #[derive(Component, Clone, Reflect)]
    struct StateTwo;
    #[derive(Component, Clone, Reflect)]
    struct StateThree;

    #[derive(Resource)]
    struct SomeResource;

    /// Triggers when SomeResource is present.
    #[derive(Reflect)]
    struct ResourcePresent;

    impl BoolTrigger for ResourcePresent {
        type Param<'w, 's> = Option<Res<'w, SomeResource>>;

        fn trigger(&self, _entity: Entity, param: &Self::Param<'_, '_>) -> bool {
            param.is_some()
        }
    }

    #[test]
    fn test_machine() {
        let machine = StateMachine::new(StateOne)
            .trans::<StateOne>(AlwaysTrigger, StateTwo)
            .trans::<StateTwo>(ResourcePresent, StateThree);
        let mut app = App::new();
        app.add_system(transition_system);

        let entity = app.world.spawn((StateOne, machine)).id();
        app.update();
        app.update();
        // should have moved to state two
        assert!(app.world.get::<StateOne>(entity).is_none());
        assert!(app.world.get::<StateTwo>(entity).is_some());

        app.update();
        // not yet...
        assert!(app.world.get::<StateTwo>(entity).is_some());
        assert!(app.world.get::<StateThree>(entity).is_none());

        app.world.insert_resource(SomeResource);
        app.update();
        // okay, *now*
        assert!(app.world.get::<StateTwo>(entity).is_none());
        assert!(app.world.get::<StateThree>(entity).is_some());
    }

    #[test]
    fn test_self_transition() {
        let machine = StateMachine::new(StateOne).trans::<StateOne>(AlwaysTrigger, StateOne);
        let mut app = App::new();
        app.add_system(transition_system);

        let entity = app.world.spawn((StateOne, machine)).id();
        app.update();
        // the sort of bug this is trying to catch: if you insert the new state
        // and then remove the old state, self-transitions will leave you
        // without the state
        assert!(
            app.world.get::<StateOne>(entity).is_some(),
            "transitioning from a state to itself should work"
        );
    }
}
