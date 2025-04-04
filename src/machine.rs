//! Module for the [`StateMachine`] component

use std::{
    any::{type_name, Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
};

use bevy::{
    ecs::{intern::Interned, schedule::ScheduleLabel, system::EntityCommands, world::Command},
    utils::TypeIdMap,
};

use crate::{
    prelude::*,
    set::StateSet,
    state::OnEvent,
    trigger::{IntoTrigger, TriggerOut},
};

pub(crate) fn plug(schedule: Interned<dyn ScheduleLabel>) -> impl Fn(&mut App) {
    move |app| {
        app.add_systems(schedule, transition.in_set(StateSet::Transition));
    }
}

/// Performs a transition. We have a trait for this so we can erase [`TransitionImpl`]'s generics.
trait Transition: Debug + Send + Sync + 'static {
    /// Called before any call to `check`
    fn init(&mut self, world: &mut World);
    /// Checks whether the transition should be taken. `entity` is the entity that contains the
    /// state machine.
    fn check<'a>(
        &'a mut self,
        world: &World,
        entity: Entity,
    ) -> Option<(Box<dyn 'a + FnOnce(&mut World, TypeId)>, TypeId)>;
}

/// An edge in the state machine. The type parameters are the [`EntityTrigger`] that causes this
/// transition, the previous state, the function that takes the trigger's output and builds the next
/// state, and the next state itself.
struct TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: EntityTrigger,
    Prev: EntityState,
    Build: System<In = Trans<Prev, <Trig::Out as TriggerOut>::Ok>, Out = Next>,
    Next: Component + EntityState,
{
    trigger: Trig,
    builder: Build,
    phantom: PhantomData<Prev>,
}

impl<Trig, Prev, Build, Next> Debug for TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: EntityTrigger,
    Prev: EntityState,
    Build: System<In = Trans<Prev, <Trig::Out as TriggerOut>::Ok>, Out = Next>,
    Next: Component + EntityState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionImpl")
            .field("trigger", &self.trigger.type_id())
            .field("builder", &self.builder.type_id())
            .field("phantom", &self.phantom)
            .finish()
    }
}

impl<Trig, Prev, Build, Next> Transition for TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: EntityTrigger,
    Prev: EntityState,
    Build: System<In = Trans<Prev, <Trig::Out as TriggerOut>::Ok>, Out = Next>,
    Next: Component + EntityState,
{
    fn init(&mut self, world: &mut World) {
        self.trigger.init(world);
        self.builder.initialize(world);
    }

    fn check<'a>(
        &'a mut self,
        world: &World,
        entity: Entity,
    ) -> Option<(Box<dyn 'a + FnOnce(&mut World, TypeId)>, TypeId)> {
        self.trigger
            .check(entity, world)
            .into_result()
            .map(|out| {
                (
                    Box::new(move |world: &mut World, curr: TypeId| {
                        let prev = Prev::remove(entity, world, curr);
                        let next = self.builder.run(TransCtx { prev, out, entity }, world);
                        world.entity_mut(entity).insert(next);
                    }) as Box<dyn 'a + FnOnce(&mut World, TypeId)>,
                    TypeId::of::<Next>(),
                )
            })
            .ok()
    }
}

impl<Trig, Prev, Build, Next> TransitionImpl<Trig, Prev, Build, Next>
where
    Trig: EntityTrigger,
    Prev: EntityState,
    Build: System<In = Trans<Prev, <Trig::Out as TriggerOut>::Ok>, Out = Next>,
    Next: Component + EntityState,
{
    pub fn new(trigger: Trig, builder: Build) -> Self {
        Self {
            trigger,
            builder,
            phantom: PhantomData,
        }
    }
}

/// Context for a transition
pub struct TransCtx<Prev, Out> {
    /// Previous state
    pub prev: Prev,
    /// Output from the trigger
    pub out: Out,
    /// The entity with this state machine
    pub entity: Entity,
}

/// Context for a transition, usable as a `SystemInput`
pub type Trans<Prev, Out> = In<TransCtx<Prev, Out>>;

/// Information about a state
#[derive(Debug)]
struct StateMetadata {
    /// For debug information
    name: String,
}

impl StateMetadata {
    fn new<S: EntityState>() -> Self {
        Self {
            name: type_name::<S>().to_string(),
        }
    }
}

/// State machine component.
///
/// Entities with this component will have components (the states) added
/// and removed based on the transitions that you add. Build one with `StateMachine::default`,
/// `StateMachine::trans`, and other methods.
#[derive(Component)]
pub struct StateMachine {
    states: TypeIdMap<StateMetadata>,
    /// Each transition and the state it should apply in (or [`AnyState`]). We store the transitions
    /// in a flat list so that we ensure we always check them in the right order; storing them in
    /// each StateMetadata would mean that e.g. we'd have to check every AnyState trigger before any
    /// state-specific trigger or vice versa.
    transitions: Vec<(fn(TypeId) -> bool, Box<dyn Transition>)>,
    on_exit: Vec<(fn(TypeId) -> bool, fn(TypeId) -> bool, OnEvent)>,
    on_enter: Vec<(fn(TypeId) -> bool, fn(TypeId) -> bool, OnEvent)>,
    /// Transitions must be initialized whenever a transition is added or a transition occurs
    init_transitions: bool,
    /// If true, all transitions are logged at info level
    log_transitions: bool,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self {
            states: default(),
            transitions: Vec::new(),
            on_exit: Vec::new(),
            on_enter: Vec::new(),
            init_transitions: true,
            log_transitions: false,
        }
    }
}

impl StateMachine {
    /// Registers a state. This is only necessary for states that are not used in any transitions.
    pub fn with_state<S: Clone + Component>(mut self) -> Self {
        self.metadata_mut::<S>();
        self
    }

    /// Adds a transition to the state machine. When the entity is in the state given as a
    /// type parameter, and the given trigger occurs, it will transition to the state given as a
    /// function parameter. Elide the `Marker` type parameter with `_`. Transitions have priority
    /// in the order they are added.
    pub fn trans<S: EntityState, Marker>(
        self,
        trigger: impl IntoTrigger<Marker>,
        state: impl Clone + Component,
    ) -> Self {
        self.trans_builder(trigger, move |_: Trans<S, _>| state.clone())
    }

    /// Get the metadata for the given state, creating it if necessary.
    fn metadata_mut<S: EntityState>(&mut self) -> &mut StateMetadata {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
    }

    /// Adds a transition builder to the state machine. When the entity is in `Prev` state, and
    /// `Trig` occurs, the given builder will be run on `Trig::Ok`. If the builder returns
    /// `Some(Next)`, the machine will transition to that `Next` state.
    pub fn trans_builder<
        Prev: EntityState,
        Trig: IntoTrigger<TrigMarker>,
        Next: Clone + Component,
        TrigMarker,
        BuildMarker,
    >(
        mut self,
        trigger: Trig,
        builder: impl IntoSystem<
            Trans<Prev, <<Trig::Trigger as EntityTrigger>::Out as TriggerOut>::Ok>,
            Next,
            BuildMarker,
        >,
    ) -> Self {
        self.metadata_mut::<Prev>();
        self.metadata_mut::<Next>();
        let transition = TransitionImpl::<_, Prev, _, _>::new(
            trigger.into_trigger(),
            IntoSystem::into_system(builder),
        );
        self.transitions
            .push((Prev::matches, Box::new(transition) as Box<dyn Transition>));
        self.init_transitions = true;
        self
    }

    /// Adds an on-enter event to the state machine. Whenever the state machine transitions to the
    /// given next state from the given current state, it will run the event.
    pub fn on_enter<NextState: EntityState, CurrentState: EntityState>(
        mut self,
        on_enter: impl 'static + Fn(&mut EntityCommands) + Send + Sync,
    ) -> Self {
        self.on_enter.push((
            NextState::matches,
            CurrentState::matches,
            OnEvent::Entity(Box::new(on_enter)),
        ));

        self
    }

    /// Adds an on-exit event to the state machine. Whenever the state machine transitions from the
    /// given current state to the given next state, it will run the event.
    pub fn on_exit<CurrentState: EntityState, NextState: EntityState>(
        mut self,
        on_exit: impl 'static + Fn(&mut EntityCommands) + Send + Sync,
    ) -> Self {
        self.on_exit.push((
            CurrentState::matches,
            NextState::matches,
            OnEvent::Entity(Box::new(on_exit)),
        ));

        self
    }

    /// Adds an on-enter command to the state machine. Whenever the state machine transitions from the
    /// given next state from the given current state, it will run the command.
    pub fn command_on_enter<NextState: EntityState, CurrentState: EntityState>(
        mut self,
        command: impl Clone + Command + Sync,
    ) -> Self {
        self.on_enter.push((
            NextState::matches,
            CurrentState::matches,
            OnEvent::Command(Box::new(command)),
        ));

        self
    }

    /// Adds an on-exit command to the state machine. Whenever the state machine transitions from the
    /// given curent stateto the given next state, it will run the command.
    pub fn command_on_exit<CurrentState: EntityState, NextState: EntityState>(
        mut self,
        command: impl Clone + Command + Sync,
    ) -> Self {
        self.on_exit.push((
            CurrentState::matches,
            NextState::matches,
            OnEvent::Command(Box::new(command)),
        ));

        self
    }

    /// Sets whether transitions are logged to the console
    pub fn set_trans_logging(mut self, log_transitions: bool) -> Self {
        self.log_transitions = log_transitions;
        self
    }

    /// Initialize all transitions. Must be executed before `run`. This is separate because `run` is
    /// parallelizable (takes a `&World`) but this isn't (takes a `&mut World`).
    fn init_transitions(&mut self, world: &mut World) {
        if !self.init_transitions {
            return;
        }

        for (_, transition) in &mut self.transitions {
            transition.init(world);
        }

        self.init_transitions = false;
    }

    /// Runs all transitions until one is actually taken. If one is taken, logs the transition and
    /// runs `on_enter/on_exit` triggers.
    // TODO Defer the actual transition so this can be parallelized, and see if that improves perf
    fn run(&mut self, world: &mut World, entity: Entity) {
        let mut states = self.states.keys();
        let current = states.find(|&&state| world.entity(entity).contains_type_id(state));

        let Some(&current) = current else {
            error!("Entity {entity:?} is in no state");
            return;
        };

        let from = &self.states[&current];
        if let Some(&other) = states.find(|&&state| world.entity(entity).contains_type_id(state)) {
            let state = &from.name;
            let other = &self.states[&other].name;
            error!("{entity:?} is in multiple states: {state} and {other}");
            return;
        }

        let Some((trans, next_state)) = self
            .transitions
            .iter_mut()
            .filter(|(matches, _)| matches(current))
            .find_map(|(_, transition)| transition.check(world, entity))
        else {
            return;
        };
        let to = &self.states[&next_state];

        for (matches_current, matches_next, event) in &self.on_exit {
            if matches_current(current) && matches_next(next_state) {
                event.trigger(entity, &mut world.commands());
            }
        }

        trans(world, current);

        for (matches_next, matches_current, event) in &self.on_enter {
            if matches_next(next_state) && matches_current(current) {
                event.trigger(entity, &mut world.commands());
            }
        }

        if self.log_transitions {
            info!("{entity:?} transitioned from {} to {}", from.name, to.name);
        }

        self.init_transitions = true;
    }
}

/// Runs all transitions on all entities.
// There are comments here about parallelization, but this is not parallelized anymore. Leaving them
// here in case it gets parallelized again.
pub(crate) fn transition(
    world: &mut World,
    machine_query: &mut QueryState<(Entity, &mut StateMachine)>,
) {
    // Pull the machines out of the world so we can invoke mutable methods on them. The alternative
    // would be to wrap the entire `StateMachine` in an `Arc<Mutex>`, but that would complicate the
    // API surface and you wouldn't be able to do anything more anyway (since you'd need to lock the
    // mutex anyway).
    let mut borrowed_machines: Vec<(Entity, StateMachine)> = machine_query
        .iter_mut(world)
        .map(|(entity, mut machine)| {
            let stub = StateMachine::default();
            (entity, std::mem::replace(machine.as_mut(), stub))
        })
        .collect();

    // `world` is mutable here, since initialization requires mutating the world
    for (_, machine) in borrowed_machines.iter_mut() {
        machine.init_transitions(world);
    }

    // `world` is not mutated here; the state machines are not in the world, and the Commands don't
    // mutate until application
    // let par_commands = system_state.get(world);
    // let task_pool = ComputeTaskPool::get();

    // chunk size of None means to automatically pick
    for &mut (entity, ref mut machine) in &mut borrowed_machines {
        machine.run(world, entity);
    }

    // put the borrowed machines back
    for (entity, borrowed_machine) in borrowed_machines {
        let Ok((_, mut machine)) = machine_query.get_mut(world, entity) else {
            // The `StateMachine` component was removed in a transition
            continue;
        };

        *machine = borrowed_machine;
    }

    // necessary to actually *apply* the commands we've enqueued
    // system_state.apply(world);
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

    /// Triggers when `SomeResource` is present
    fn resource_present(res: Option<Res<SomeResource>>) -> bool {
        res.is_some()
    }

    #[test]
    fn test_sets_initial_state() {
        let mut app = App::new();
        app.add_systems(Update, transition);
        let machine = StateMachine::default().with_state::<StateOne>();
        let entity = app.world_mut().spawn((machine, StateOne)).id();
        app.update();
        // should have moved to state two
        assert!(
            app.world().get::<StateOne>(entity).is_some(),
            "StateMachine should have the initial component"
        );
    }

    #[test]
    fn test_machine() {
        let mut app = App::new();
        app.add_systems(Update, transition);

        let machine = StateMachine::default()
            .trans::<StateOne, _>(
                Box::new(always.into_trigger()) as Box<dyn EntityTrigger<Out = _>>,
                StateTwo,
            )
            .trans::<StateTwo, _>(resource_present, StateThree);
        let entity = app.world_mut().spawn((machine, StateOne)).id();

        assert!(app.world().get::<StateOne>(entity).is_some());

        app.update();
        // should have moved to state two
        assert!(app.world().get::<StateOne>(entity).is_none());
        assert!(app.world().get::<StateTwo>(entity).is_some());

        app.update();
        // not yet...
        assert!(app.world().get::<StateTwo>(entity).is_some());
        assert!(app.world().get::<StateThree>(entity).is_none());

        app.world_mut().insert_resource(SomeResource);
        app.update();
        // okay, *now*
        assert!(app.world().get::<StateTwo>(entity).is_none());
        assert!(app.world().get::<StateThree>(entity).is_some());
    }

    #[test]
    fn test_self_transition() {
        let mut app = App::new();
        app.add_systems(Update, transition);

        let entity = app
            .world_mut()
            .spawn((
                StateMachine::default().trans::<StateOne, _>(always, StateOne),
                StateOne,
            ))
            .id();
        app.update();
        // the sort of bug this is trying to catch: if you insert the new state and then remove the
        // old state, self-transitions will leave you without the state
        assert!(
            app.world().get::<StateOne>(entity).is_some(),
            "transitioning from a state to itself should work"
        );
    }

    #[test]
    fn test_state_machine() {
        #[derive(Resource, Default)]
        struct Test {
            on_b: bool,
            on_any: bool,
        }

        #[derive(Clone, Debug)]
        struct MyCommand {
            on_any: bool,
        }

        impl Command for MyCommand {
            fn apply(self, world: &mut World) {
                let mut test = world.resource_mut::<Test>();
                if self.on_any {
                    test.on_any = true;
                } else {
                    test.on_b = true;
                }
            }
        }

        #[derive(Component, Clone)]
        struct A;
        #[derive(Component, Clone)]
        struct B;
        #[derive(Component, Clone)]
        struct C;
        #[derive(Component, Clone)]
        struct D;

        let mut app = App::new();
        app.init_resource::<Test>();
        app.add_plugins((
            MinimalPlugins,
            bevy::log::LogPlugin::default(),
            StateMachinePlugin::default(),
        ));
        app.update();

        let machine = StateMachine::default()
            .trans::<A, _>(always, B)
            .trans::<B, _>(always, C)
            .trans::<C, _>(always, D)
            .command_on_enter::<B, AnyState>(MyCommand { on_any: false })
            .command_on_enter::<AnyState, AnyState>(MyCommand { on_any: true })
            .set_trans_logging(true);

        let id = app.world_mut().spawn((A, machine)).id();
        app.update();
        app.update();
        app.update();
        assert!(app.world().get::<A>(id).is_none());
        assert!(app.world().get::<B>(id).is_none());
        assert!(app.world().get::<C>(id).is_none());
        assert!(app.world().get::<D>(id).is_some());

        let test = app.world().resource::<Test>();
        assert!(test.on_b, "on_b should be true");
        assert!(test.on_any, "on_any should be true");
    }

    #[test]
    fn test_event_matches() {
        #[derive(Component, Default)]
        struct InB;

        #[derive(Component, Clone)]
        struct A;

        #[derive(Component, Clone)]
        struct B1;

        #[derive(Component, Clone)]
        struct B2;

        #[derive(Component, Clone)]
        struct C;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::log::LogPlugin::default(),
            StateMachinePlugin::default(),
        ));
        app.update();

        let machine = StateMachine::default()
            .trans::<A, _>(always, B1)
            .trans::<B1, _>(always, B2)
            .trans::<B2, _>(always, C)
            .on_enter::<OneOfState<(B1, B2)>, NotState<OneOfState<(B1, B2)>>>(|ec| {
                ec.insert(InB);
            })
            .on_exit::<OneOfState<(B1, B2)>, NotState<OneOfState<(B1, B2)>>>(|ec| {
                ec.remove::<InB>();
            })
            .set_trans_logging(true);

        let id = app.world_mut().spawn((A, machine)).id();
        app.update();
        assert!(app.world().get::<InB>(id).is_some());

        app.update();
        assert!(app.world().get::<InB>(id).is_some());

        app.update();
        assert!(app.world().get::<InB>(id).is_none());
    }
}
