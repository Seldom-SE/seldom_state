use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy::ecs::system::SystemState;
use bevy::ecs::system::{Command, EntityCommands};
use bevy::tasks::{ComputeTaskPool, ParallelSliceMut};

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
    Prev: Bundle,
    Build: Fn(Trig::Ok) -> Option<Next>,
    Next: Bundle,
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
    Prev: Bundle,
    Build: Fn(Trig::Ok) -> Option<Next> + Send + Sync + 'static,
    Next: Bundle,
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

/// Stores information about each state type.
#[derive(Debug)]
struct StateMetadata {
    /// Used for better debug information.
    name: String,
    on_enter: Vec<OnEvent>,
    on_exit: Vec<OnEvent>,
}

impl StateMetadata {
    fn new<S: Bundle>() -> Self {
        Self {
            name: type_name::<S>().to_owned(),
            on_enter: default(),
            on_exit: default(),
        }
    }
}

/// The state of a StateMachine before it's actually run. We have an explicit
/// state for this so that we can make sure that on_enter triggers and so on
/// run properly.
#[derive(Clone, Component, Reflect)]
struct InitialState;

/// State machine component. Entities with this component will have bundles (the states)
/// added and removed based on the transitions that you add. Build one
/// with `StateMachine::new`, `StateMachine::trans`, and other methods.
#[derive(Component)]
pub struct StateMachine {
    /// TypeId of the current state.
    current: TypeId,
    states: HashMap<TypeId, StateMetadata>,
    /// Each transition and the state it should apply in (or [`AnyState`]). We
    /// store the transitions in a flat list so that we ensure we always check
    /// them in the right order; storing them in each StateMetadata would mean
    /// that e.g. we'd have to check every AnyState trigger before any
    /// state-specific trigger or vice versa.
    transitions: Vec<(TypeId, Box<dyn Transition>)>,
    /// If true, all transitions are logged at info level.
    log_transitions: bool,
}

impl StateMachine {
    /// Creates a state machine with an initial state and no transitions. Add more
    /// with [`StateMachine::trans`].
    pub fn new<S: MachineState>(initial: S) -> Self {
        let machine = Self {
            current: TypeId::of::<InitialState>(),
            states: HashMap::from([(TypeId::of::<AnyState>(), StateMetadata::new::<AnyState>())]),
            transitions: vec![],
            log_transitions: false,
        };
        machine.trans::<InitialState>(AlwaysTrigger, initial)
    }

    /// Get the medatada for the given state, creating it if necessary.
    fn metadata_mut<S: MachineState>(&mut self) -> &mut StateMetadata {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
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
        self.metadata_mut::<P>();
        self.metadata_mut::<N>();
        let transition: TransitionImpl<T, P, _, N> = TransitionImpl::new(trigger, builder);
        self.transitions.push((
            TypeId::of::<P>(),
            Box::new(transition) as Box<dyn Transition>,
        ));
        self
    }

    /// Adds an on-enter event to the state machine. Whenever the state machine transitions
    /// into the given state, it will run the event.
    pub fn on_enter<S: MachineState>(
        mut self,
        on_enter: impl 'static + Fn(&mut EntityCommands) + Send + Sync,
    ) -> Self {
        self.metadata_mut::<S>()
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
        self.metadata_mut::<S>()
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
        self.metadata_mut::<S>()
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
        self.metadata_mut::<S>()
            .on_exit
            .push(OnEvent::Command(Box::new(command)));

        self
    }

    /// Sets whether transitions are logged to the console
    pub fn set_trans_logging(mut self, log_transitions: bool) -> Self {
        self.log_transitions = log_transitions;
        self
    }

    /// Get the list of transitions for the current state.
    fn transitions_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn Transition>> {
        self.transitions
            .iter_mut()
            .filter(|(type_id, _)| *type_id == self.current || *type_id == TypeId::of::<AnyState>())
            .map(|(_, transition)| transition)
    }

    /// Initialize all transitions. Must be executed before `run`. This is
    /// separate because `run` is parallelizable (takes a `&World`) but this
    /// isn't (takes a `&mut World`).
    fn initialize(&mut self, world: &mut World) {
        for transition in self.transitions_mut() {
            transition.initialize(world);
        }
    }

    /// Runs all transitions until one is actually taken. If one is taken, logs
    /// the transition and runs `on_enter/on_exit` triggers.
    fn run(&mut self, world: &World, entity: Entity, commands: &mut Commands) {
        let next_state = self
            .transitions_mut()
            .find_map(|transition| transition.run(world, &mut commands.entity(entity)));
        if let Some(next_state) = next_state {
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

    /// When running the transition system, we replace all StateMachines in the
    /// world with their stub.
    fn stub(&self) -> Self {
        Self {
            current: self.current,
            states: default(),
            log_transitions: false,
            transitions: default(),
        }
    }
}

/// Runs all transitions on all entities.
pub(crate) fn transition_system(
    world: &mut World,
    system_state: &mut SystemState<ParallelCommands>,
    machine_query: &mut QueryState<(Entity, &mut StateMachine)>,
) {
    // Pull the machines out of the world so we can invoke mutable methods on
    // them. The alternative would be to wrap the entire StateMachine in an
    // Arc<Mutex<>>, but that would complicate the API surface and you wouldn't
    // be able to do anything more anyway (since you'd need to lock the mutex
    // anyway).
    let mut borrowed_machines: Vec<(Entity, StateMachine)> = machine_query
        .iter_mut(world)
        .map(|(entity, mut machine)| {
            let stub = machine.stub();
            (entity, std::mem::replace(machine.as_mut(), stub))
        })
        .collect();

    // world is mutable here, since initialization require mutating the world
    for (_, machine) in borrowed_machines.iter_mut() {
        machine.initialize(world);
    }

    // world is not mutated here; the state machines are not in the world, and
    // the Commands don't mutate until application.
    let par_commands = system_state.get(world);
    let task_pool = ComputeTaskPool::get();
    // chunk size of None means to automatically pick
    borrowed_machines.par_splat_map_mut(task_pool, None, |chunk| {
        for (entity, machine) in chunk {
            par_commands.command_scope(|mut commands| machine.run(world, *entity, &mut commands));
        }
    });

    // put the borrowed machines back
    for (entity, machine) in borrowed_machines {
        *machine_query.get_mut(world, entity).unwrap().1 = machine;
    }

    // necessary to actually *apply* the commands we've enqueued
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
    fn test_sets_initial_state() {
        let mut app = App::new();
        app.add_system(transition_system);
        let machine = StateMachine::new(StateOne);
        let entity = app.world.spawn(machine).id();
        app.update();
        // should have moved to state two
        assert!(
            app.world.get::<StateOne>(entity).is_some(),
            "StateMachine should insert the initial component"
        );
    }

    #[test]
    fn test_machine() {
        let mut app = App::new();
        app.add_system(transition_system);

        let machine = StateMachine::new(StateOne)
            .trans::<StateOne>(AlwaysTrigger, StateTwo)
            .trans::<StateTwo>(ResourcePresent, StateThree);
        let entity = app.world.spawn(machine).id();

        app.update();
        assert!(app.world.get::<StateOne>(entity).is_some());

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
        let mut app = App::new();
        app.add_system(transition_system);

        let entity = app
            .world
            .spawn(StateMachine::new(StateOne).trans::<StateOne>(AlwaysTrigger, StateOne))
            .id();
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
