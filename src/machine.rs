use std::{
    any::{type_name, Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
};

use bevy::{
    ecs::system::{Command, EntityCommands, SystemState},
    tasks::{ComputeTaskPool, ParallelSliceMut},
    utils::HashMap,
};

use crate::{
    prelude::*,
    set::StateSet,
    state::{Insert, OnEvent},
};

pub(crate) fn machine_plugin(app: &mut App) {
    app.configure_set(StateSet::Transition.in_base_set(CoreSet::PostUpdate))
        .add_system(transition.in_set(StateSet::Transition));
}

/// Performs a transition. We have a trait for this so we can erase [`TransitionImpl`]'s generics.
trait Transition: Debug + Send + Sync + 'static {
    /// Called before any call to `run`.
    fn init(&mut self, world: &mut World);
    /// Checks whether the transition should be taken. `entity` is the entity
    /// that contains the state machine.
    fn run(&mut self, world: &World, entity: Entity) -> Option<(Box<dyn Insert>, TypeId)>;
}

/// An edge in the state machine. The type parameters are
/// the [`Trigger`] that causes this transition, the previous state the function that
/// takes the trigger's output and builds the next state, and the next state itself.
struct TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Prev: MachineState,
    Build: 'static + Fn(Trig::Ok) -> Option<Next> + Send + Sync,
    Next: MachineState,
{
    pub trigger: Trig,
    pub builder: Build,
    // To run this, we need a [`SystemState`]. We can't initialize
    // that until we have a [`World`], so it starts out empty.
    system_state: Option<SystemState<Trig::Param<'static, 'static>>>,
    phantom: PhantomData<Prev>,
}

impl<Trig, Prev, Build, Next> Debug for TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Prev: MachineState,
    Build: Fn(Trig::Ok) -> Option<Next> + Send + Sync,
    Next: MachineState,
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

impl<Trig, Prev, Build, Next> Transition for TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Prev: MachineState,
    Build: Fn(Trig::Ok) -> Option<Next> + Send + Sync,
    Next: MachineState,
{
    fn init(&mut self, world: &mut World) {
        if self.system_state.is_none() {
            self.system_state = Some(SystemState::new(world));
        }
    }

    fn run(&mut self, world: &World, entity: Entity) -> Option<(Box<dyn Insert>, TypeId)> {
        let state = self.system_state.as_mut().unwrap();
        let Ok(res) = self.trigger.trigger(entity, &state.get(world)) else { return None };
        (self.builder)(res).map(|state| (Box::new(state) as Box<dyn Insert>, TypeId::of::<Next>()))
    }
}

impl<Trig, Prev, Build, Next> TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: Trigger,
    Prev: MachineState,
    Build: Fn(Trig::Ok) -> Option<Next> + Send + Sync,
    Next: MachineState,
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

/// Information about a state
#[derive(Debug)]
struct StateMetadata {
    /// For debug information
    name: String,
    on_enter: Vec<OnEvent>,
    on_exit: Vec<OnEvent>,
}

impl StateMetadata {
    fn new<S: Bundle>() -> Self {
        Self {
            name: type_name::<S>().to_owned(),
            on_enter: default(),
            on_exit: vec![OnEvent::Entity(Box::new(|entity: &mut EntityCommands| {
                entity.remove::<S>();
            }))],
        }
    }
}

/// State machine component. Entities with this component will have bundles (the states)
/// added and removed based on the transitions that you add. Build one
/// with `StateMachine::new`, `StateMachine::trans`, and other methods.
#[derive(Component)]
pub struct StateMachine {
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

impl Default for StateMachine {
    fn default() -> Self {
        Self {
            states: HashMap::from([(TypeId::of::<AnyState>(), StateMetadata::new::<AnyState>())]),
            transitions: vec![],
            log_transitions: false,
        }
    }
}

impl StateMachine {
    /// Registers a state. This is only necessary for states that are not used in any transitions.
    pub fn with_state<S: MachineState>(mut self) -> Self {
        self.metadata_mut::<S>();
        self
    }

    /// Adds a transition to the state machine. When the entity is in the state
    /// given as a type parameter, and the given trigger occurs, it will transition
    /// to the state given as a function parameter.
    pub fn trans<S: MachineState>(self, trigger: impl Trigger, state: impl MachineState) -> Self {
        self.trans_builder::<S, _, _>(trigger, move |_| Some(state.clone()))
    }

    /// Get the medatada for the given state, creating it if necessary.
    fn metadata_mut<S: MachineState>(&mut self) -> &mut StateMetadata {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
    }

    /// Adds a transition builder to the state machine. When the entity is in `P` state, and `T`
    /// occurs, the given builder will be run on `T`'s `Result` type. If the builder returns
    /// `Some(N)`, the machine will transition to that `N` state.
    pub fn trans_builder<Prev: MachineState, Trig: Trigger, Next: MachineState>(
        mut self,
        trigger: Trig,
        builder: impl 'static + Clone + Fn(Trig::Ok) -> Option<Next> + Send + Sync,
    ) -> Self {
        self.metadata_mut::<Prev>();
        self.metadata_mut::<Next>();
        let transition = TransitionImpl::<_, Prev, _, _>::new(trigger, builder);
        self.transitions.push((
            TypeId::of::<Prev>(),
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

    /// Initialize all transitions. Must be executed before `run`. This is
    /// separate because `run` is parallelizable (takes a `&World`) but this
    /// isn't (takes a `&mut World`).
    fn init_transitions(&mut self, world: &mut World) {
        for (_, transition) in &mut self.transitions {
            transition.init(world);
        }
    }

    /// Runs all transitions until one is actually taken. If one is taken, logs
    /// the transition and runs `on_enter/on_exit` triggers.
    fn run(&mut self, world: &World, entity: Entity, commands: &mut Commands) {
        let mut states = self.states.keys();
        let current = states.find(|&&state| world.entity(entity).contains_type_id(state));

        let Some(&current) = current else {
            panic!("Entity {entity:?} is in no state");
        };

        let from = &self.states[&current];
        if let Some(&other) = states.find(|&&state| world.entity(entity).contains_type_id(state)) {
            let state = &from.name;
            let other = &self.states[&other].name;
            panic!("{entity:?} is in multiple states: {state} and {other}");
        }

        let Some((insert, next_state)) = self
            .transitions
            .par_splat_map_mut(ComputeTaskPool::get(), None, |transitions| {
                transitions
                    .iter_mut()
                    .filter(|(type_id, _)| {
                        *type_id == current || *type_id == TypeId::of::<AnyState>()
                    })
                    .find_map(|(_, transition)| transition.run(world, entity))
            })
            .into_iter()
            .find_map(|x| x) else { return };
        let to = &self.states[&next_state];

        for event in from.on_exit.iter() {
            event.trigger(entity, commands);
        }

        insert.insert(&mut commands.entity(entity));
        for event in to.on_enter.iter() {
            event.trigger(entity, commands);
        }

        if self.log_transitions {
            info!("{entity:?} transitioned from {} to {}", from.name, to.name);
        }
    }

    /// When running the transition system, we replace all StateMachines in the
    /// world with their stub.
    fn stub(&self) -> Self {
        Self {
            states: default(),
            log_transitions: false,
            transitions: default(),
        }
    }
}

/// Runs all transitions on all entities.
pub(crate) fn transition(
    world: &mut World,
    system_state: &mut SystemState<ParallelCommands>,
    machine_query: &mut QueryState<(Entity, &mut StateMachine)>,
) {
    // Pull the machines out of the world so we can invoke mutable methods on
    // them. The alternative would be to wrap the entire `StateMachine` in an
    // `Arc<Mutex>`, but that would complicate the API surface and you wouldn't
    // be able to do anything more anyway (since you'd need to lock the mutex
    // anyway).
    let mut borrowed_machines: Vec<(Entity, StateMachine)> = machine_query
        .iter_mut(world)
        .map(|(entity, mut machine)| {
            let stub = machine.stub();
            (entity, std::mem::replace(machine.as_mut(), stub))
        })
        .collect();

    // `world` is mutable here, since initialization requires mutating the world
    for (_, machine) in borrowed_machines.iter_mut() {
        machine.init_transitions(world);
    }

    // `world` is not mutated here; the state machines are not in the world, and
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

    // Test states to transition between.
    #[derive(Component, Clone)]
    struct StateOne;
    #[derive(Component, Clone)]
    struct StateTwo;
    #[derive(Component, Clone)]
    struct StateThree;

    #[derive(Resource)]
    struct SomeResource;

    /// Triggers when `SomeResource` is present.
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
        app.add_system(transition);
        let machine = StateMachine::default().with_state::<StateOne>();
        let entity = app.world.spawn((machine, StateOne)).id();
        app.update();
        // should have moved to state two
        assert!(
            app.world.get::<StateOne>(entity).is_some(),
            "StateMachine should have the initial component"
        );
    }

    #[test]
    fn test_machine() {
        let mut app = App::new();
        app.add_system(transition);

        let machine = StateMachine::default()
            .trans::<StateOne>(AlwaysTrigger, StateTwo)
            .trans::<StateTwo>(ResourcePresent, StateThree);
        let entity = app.world.spawn((machine, StateOne)).id();

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
        app.add_system(transition);

        let entity = app
            .world
            .spawn((
                StateMachine::default().trans::<StateOne>(AlwaysTrigger, StateOne),
                StateOne,
            ))
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
