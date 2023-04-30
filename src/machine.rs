use std::any::{type_name, TypeId};

use bevy::{
    ecs::system::{Command, EntityCommands},
    utils::HashMap,
};

use crate::{
    prelude::*,
    set::StateSet,
    state::{AsDynStateBuilderTypedBox, DynState, OnEvent, StateBuilder},
    trigger::{DynTrigger, RegisteredTriggers},
};

pub(crate) fn machine_plugin(app: &mut App) {
    app.add_system(transition.in_set(StateSet::Transition));
}

#[derive(Debug)]
enum TransitionTarget {
    State(Box<dyn DynState>),
    Builder(Box<dyn StateBuilder>),
}

impl TransitionTarget {
    fn state(&self, result: &dyn Reflect) -> Option<Box<dyn DynState>> {
        match self {
            Self::State(state) => Some(state.dyn_clone()),
            Self::Builder(builder) => builder.build(result),
        }
    }
}

#[derive(Debug)]
struct Transition {
    result: Option<Box<dyn Reflect>>,
    trigger: Box<dyn DynTrigger>,
    target: TransitionTarget,
}

#[derive(Debug)]
struct TriggerTransitions {
    transitions: Vec<Transition>,
    name: String,
}

#[derive(Debug, Default)]
struct Transitions {
    transitions: Vec<TriggerTransitions>,
    trigger_indices: HashMap<TypeId, usize>,
}

impl Transitions {
    fn add(
        &mut self,
        trigger: Box<dyn DynTrigger>,
        trigger_id: TypeId,
        trigger_name: String,
        target: TransitionTarget,
    ) {
        let transition = Transition {
            result: None,
            trigger,
            target,
        };

        if let Some(index) = self.trigger_indices.get(&trigger_id) {
            self.transitions[*index].transitions.push(transition);
        } else {
            self.trigger_indices
                .insert(trigger_id, self.transitions.len());
            self.transitions.push(TriggerTransitions {
                transitions: vec![transition],
                name: trigger_name,
            });
        }
    }
}

#[derive(Debug)]
struct StateMetadata {
    name: String,
    transitions: Transitions,
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

    fn new_of<S: Bundle>(_: &S) -> Self {
        Self::new::<S>()
    }
}

/// State machine component. Entities with this component will have bundles (the states)
/// added and removed based on the transitions that you add. Build one
/// with `StateMachine::new`, `StateMachine::trans`, and other methods.
#[derive(Component, Debug)]
pub struct StateMachine {
    current: Option<TypeId>,
    states: HashMap<TypeId, StateMetadata>,
    log_transitions: bool,
}

impl StateMachine {
    /// Creates a state machine with an initial state and no transitions. Add more
    /// with [`StateMachine::trans`].
    pub fn new(initial: impl MachineState) -> Self {
        let type_id = initial.type_id();
        let metadata = StateMetadata::new_of(&initial);

        let mut initial_metadata = StateMetadata::new::<()>();
        initial_metadata.transitions.add(
            Box::new(AlwaysTrigger),
            TypeId::of::<AlwaysTrigger>(),
            AlwaysTrigger.base_type_name().to_string(),
            TransitionTarget::State(Box::new(initial)),
        );

        Self {
            current: None,
            states: vec![
                (TypeId::of::<bool>(), initial_metadata),
                (type_id, metadata),
                (TypeId::of::<AnyState>(), StateMetadata::new::<()>()),
            ]
            .into_iter()
            .collect(),
            log_transitions: false,
        }
    }

    fn register_state<S: MachineState>(&mut self) {
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>());
    }

    fn register_state_of<S: MachineState>(&mut self, _: &S) {
        self.register_state::<S>();
    }

    /// Adds a transition to the state machine. When the entity is in the state
    /// given as a type parameter, and the given trigger occurs, it will transition
    /// to the state given as a function parameter.
    pub fn trans<S: MachineState>(
        mut self,
        trigger: impl Trigger,
        state: impl MachineState,
    ) -> Self {
        let trigger_id = trigger.type_id();
        let name = trigger.base_type_name().to_string();

        self.register_state_of(&state);
        self.states
            .entry(TypeId::of::<S>())
            .or_insert(StateMetadata::new::<S>())
            .transitions
            .add(
                Box::new(trigger),
                trigger_id,
                name,
                TransitionTarget::State(Box::new(state)),
            );

        self
    }

    /// Adds a transition builder to the state machine. When the entity is in `P` state, and `T`
    /// occurs, the given builder will be run on `T`'s `Result` type. If the builder returns
    /// `Some(N)`, the machine will transition to that `N` state.
    pub fn trans_builder<P: MachineState, T: Trigger, N: MachineState>(
        mut self,
        trigger: T,
        builder: impl 'static + Clone + Fn(&T::Ok) -> Option<N> + Send + Sync,
    ) -> Self {
        let trigger_id = trigger.type_id();
        let name = trigger.base_type_name().to_string();
        self.states
            .entry(TypeId::of::<P>())
            .or_insert(StateMetadata::new::<P>())
            .transitions
            .add(
                Box::new(trigger),
                trigger_id,
                name,
                TransitionTarget::Builder(Box::new(Box::new(builder).as_dyn_state_builder_typed())),
            );

        self.register_state::<N>();

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

    pub(crate) fn validate_triggers(&self, registered: &RegisteredTriggers) {
        for state in self.states.values() {
            for (trigger, index) in &state.transitions.trigger_indices {
                if !registered.contains(trigger) {
                    panic!(
                        "a `StateMachine` was created with an unregistered trigger: {}",
                        state.transitions.transitions[*index].name
                    );
                }
            }
        }
    }

    fn current_id(&self) -> TypeId {
        self.current.unwrap_or(TypeId::of::<bool>())
    }

    fn current(&self) -> &StateMetadata {
        &self.states[&self.current_id()]
    }

    fn current_mut(&mut self) -> &mut StateMetadata {
        self.states.get_mut(&self.current_id()).unwrap()
    }

    pub(crate) fn get_triggers<T: Trigger>(&self, any_state: bool) -> impl IntoIterator<Item = &T> {
        let transitions = &match any_state {
            true => self.states.get(&TypeId::of::<AnyState>()).unwrap(),
            false => self.current(),
        }
        .transitions;

        transitions
            .trigger_indices
            .get(&TypeId::of::<T>())
            .map(|index| {
                transitions.transitions[*index]
                    .transitions
                    .iter()
                    .map(|transition| transition.trigger.as_reflect().downcast_ref::<T>().unwrap())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub(crate) fn mark_trigger<T: Trigger>(
        &mut self,
        index: usize,
        result: T::Ok,
        any_state: bool,
    ) {
        let transition = match any_state {
            true => self.states.get(&TypeId::of::<AnyState>()).unwrap(),
            false => self.current(),
        }
        .transitions
        .trigger_indices[&TypeId::of::<T>()];

        match any_state {
            true => self.states.get_mut(&TypeId::of::<AnyState>()).unwrap(),
            false => self.current_mut(),
        }
        .transitions
        .transitions[transition]
            .transitions[index]
            .result = Some(Box::new(result));
    }
}

fn transition(mut commands: Commands, mut machines: Query<(Entity, &mut StateMachine)>) {
    for (entity, mut machine) in &mut machines {
        let state = [machine.current_id(), TypeId::of::<AnyState>()]
            .into_iter()
            .flat_map(|id| {
                machine
                    .states
                    .get(&id)
                    .unwrap()
                    .transitions
                    .transitions
                    .iter()
                    .enumerate()
                    .map(move |(index, transitions)| (id, index, transitions))
            })
            .flat_map(|(id, trigger_index, transitions)| {
                transitions
                    .transitions
                    .iter()
                    .map(move |transition| (id, trigger_index, transition))
            })
            .find_map(|(id, index, transition)| {
                transition
                    .result
                    .as_ref()
                    .and_then(|result| transition.target.state(&**result))
                    .map(|state| (id, index, state))
            });

        for id in [machine.current_id(), TypeId::of::<AnyState>()] {
            for transitions in &mut machine.states.get_mut(&id).unwrap().transitions.transitions {
                for transition in &mut transitions.transitions {
                    transition.result = None;
                }
            }
        }

        let Some((id, index, state)) = state else { continue };

        if machine.log_transitions {
            let trigger = &machine.states.get(&id).unwrap().transitions.transitions[index].name;
            let from = &machine.current().name;
            let to = state.type_name();

            info!("{entity:?} transitioned on {trigger} from {from} to {to}",);
        }

        for on_exit in &machine.current().on_exit {
            on_exit.trigger(entity, &mut commands);
        }

        for on_exit in &machine
            .states
            .get(&TypeId::of::<AnyState>())
            .unwrap()
            .on_exit
        {
            on_exit.trigger(entity, &mut commands);
        }

        state.insert(&mut commands.entity(entity));
        machine.current = Some(state.type_id());

        for on_enter in &machine
            .states
            .get(&TypeId::of::<AnyState>())
            .unwrap()
            .on_enter
        {
            on_enter.trigger(entity, &mut commands);
        }

        for on_enter in &machine.current().on_enter {
            on_enter.trigger(entity, &mut commands);
        }
    }
}
