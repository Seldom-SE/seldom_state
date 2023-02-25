use std::any::TypeId;

use bevy::utils::HashMap;

use crate::{
    bundle::{Insert, Removable, Remove},
    prelude::*,
    stage::StateStage,
    state::{AnyState, AsDynStateBuilderTypedBox, DynState, StateBuilder},
    trigger::{DynTrigger, RegisteredTriggers},
};

pub(crate) fn machine_plugin(app: &mut App) {
    app.add_system_to_stage(StateStage::Transition, transition);
}

#[derive(Debug)]
enum TransitionTarget {
    State(Box<dyn DynState>),
    Builder(Box<dyn StateBuilder>),
}

impl Clone for TransitionTarget {
    fn clone(&self) -> Self {
        match self {
            Self::State(state) => Self::State(DynState::dyn_clone(&**state)),
            Self::Builder(builder) => Self::Builder(builder.dyn_clone()),
        }
    }
}

impl TransitionTarget {
    fn state(
        &self,
        prev_state: Option<&dyn DynState>,
        result: &dyn Reflect,
    ) -> Option<Box<dyn DynState>> {
        match self {
            Self::State(state) => Some(DynState::dyn_clone(&**state)),
            Self::Builder(builder) => builder.build(prev_state?, result),
        }
    }
}

#[derive(Debug)]
struct Transition {
    result: Option<Box<dyn Reflect>>,
    trigger: Box<dyn DynTrigger>,
    target: TransitionTarget,
}

impl Clone for Transition {
    fn clone(&self) -> Self {
        Self {
            result: self.result.as_ref().map(|result| result.clone_value()),
            trigger: self.trigger.dyn_clone(),
            target: self.target.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct TriggerTransitions {
    transitions: Vec<Transition>,
    name: String,
}

#[derive(Clone, Debug, Default)]
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

#[derive(Debug, Default)]
struct StateMetadata {
    transitions: Transitions,
    inserts: Vec<Box<dyn Insert>>,
    removes: Vec<Box<dyn Remove>>,
}

impl Clone for StateMetadata {
    fn clone(&self) -> Self {
        Self {
            transitions: self.transitions.clone(),
            inserts: self
                .inserts
                .iter()
                .map(|insert| insert.dyn_clone())
                .collect(),
            removes: self
                .removes
                .iter()
                .map(|remove| remove.dyn_clone())
                .collect(),
        }
    }
}

/// State machine component. Entities with this component will have bundles (the states)
/// added and removed based on the transitions that you add. Build one
/// with [`StateMachine::new()`], [`StateMachine::trans()`], and other methods.
#[derive(Component, Debug)]
pub struct StateMachine {
    current: Option<Box<dyn DynState>>,
    transitions: Transitions,
    removes: Vec<Box<dyn Remove>>,
    states: HashMap<TypeId, StateMetadata>,
}

impl Clone for StateMachine {
    fn clone(&self) -> Self {
        Self {
            current: self
                .current
                .as_ref()
                .map(|current| DynState::dyn_clone(&**current)),
            transitions: self.transitions.clone(),
            removes: self
                .removes
                .iter()
                .map(|remove| remove.dyn_clone())
                .collect(),
            states: self.states.clone(),
        }
    }
}

impl StateMachine {
    /// Creates a state machine with an initial state and no transitions. Add more
    /// with [`StateMachine::trans()`].
    pub fn new(initial: impl MachineState) -> Self {
        let mut transitions = Transitions::default();
        transitions.add(
            Box::new(AlwaysTrigger),
            TypeId::of::<AlwaysTrigger>(),
            AlwaysTrigger.base_type_name().to_string(),
            TransitionTarget::State(Box::new(initial)),
        );

        Self {
            current: None,
            transitions,
            removes: default(),
            states: default(),
        }
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
        self.states
            .entry(TypeId::of::<S>())
            .or_default()
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
        builder: impl 'static + Clone + Fn(&P, &T::Ok) -> Option<N> + Send + Sync,
    ) -> Self {
        let trigger_id = trigger.type_id();
        let name = trigger.base_type_name().to_string();
        self.states
            .entry(TypeId::of::<P>())
            .or_default()
            .transitions
            .add(
                Box::new(trigger),
                trigger_id,
                name,
                TransitionTarget::Builder(Box::new(Box::new(builder).as_dyn_state_builder_typed())),
            );

        self
    }

    /// Adds an on-enter event to the state machine. Whenever the state machine transitions
    /// into the given state, it will insert the given bundle into the entity.
    pub fn insert_on_enter<S: MachineState>(mut self, bundle: impl Insert) -> Self {
        self.states
            .entry(TypeId::of::<S>())
            .or_default()
            .inserts
            .push(Box::new(bundle));

        self
    }

    /// Adds an on-exit event to the state machine. Whenever the state machine transitions
    /// from the given state, it will remove the given bundle from the entity.
    pub fn remove_on_exit<S: MachineState, B: Bundle>(mut self) -> Self {
        self.states
            .entry(TypeId::of::<S>())
            .or_default()
            .removes
            .push(Box::new(B::remover()));

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

    pub(crate) fn get_triggers<T: Trigger>(&self) -> impl IntoIterator<Item = &T> {
        self.transitions
            .trigger_indices
            .get(&TypeId::of::<T>())
            .map(|index| {
                self.transitions.transitions[*index]
                    .transitions
                    .iter()
                    .map(|transition| transition.trigger.as_reflect().downcast_ref::<T>().unwrap())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub(crate) fn mark_trigger<T: Trigger>(&mut self, index: usize, result: T::Ok) {
        self.transitions.transitions[self.transitions.trigger_indices[&TypeId::of::<T>()]]
            .transitions[index]
            .result = Some(Box::new(result));
    }
}

fn transition(mut commands: Commands, mut machines: Query<(Entity, &mut StateMachine)>) {
    for (entity, mut machine) in &mut machines {
        let current = machine.current.as_deref();
        let Some(state) = machine
            .transitions
            .transitions
            .iter()
            .flat_map(|transitions| &transitions.transitions)
            .find_map(|transition| {
                transition.result.as_ref().and_then(|result| transition.target.state(current, &**result))
            })
        else { continue };

        let mut entity = commands.entity(entity);

        if let Some(current) = &machine.current {
            current.remove(&mut entity);
        }

        for remove in &machine.removes {
            remove.remove(&mut entity);
        }

        state.insert(&mut entity);
        let default_metadata = StateMetadata::default();
        let metadata = machine
            .states
            .get(&state.type_id())
            .unwrap_or(&default_metadata);

        let any_metadata = machine
            .states
            .get(&TypeId::of::<AnyState>())
            .unwrap_or(&default_metadata);

        for insert in &any_metadata.inserts {
            insert.insert(&mut entity);
        }

        for insert in &metadata.inserts {
            insert.insert(&mut entity);
        }

        let mut transitions = metadata.transitions.clone();
        for (trigger_id, index) in &any_metadata.transitions.trigger_indices {
            let trigger_transitions = &any_metadata.transitions.transitions[*index];
            for transition in &trigger_transitions.transitions {
                transitions.add(
                    transition.trigger.dyn_clone(),
                    *trigger_id,
                    trigger_transitions.name.clone(),
                    transition.target.clone(),
                );
            }
        }

        let removes = metadata
            .removes
            .iter()
            .chain(any_metadata.removes.iter())
            .map(|remove| remove.dyn_clone())
            .collect();

        machine.current = Some(DynState::dyn_clone(&*state));
        machine.transitions = transitions;
        machine.removes = removes;
    }
}
