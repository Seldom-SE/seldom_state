use std::any::TypeId;

use bevy::utils::HashMap;

use crate::{
    bundle::{Insert, Removable, Remove},
    prelude::*,
    stage::StateStage,
    state::StateTransition,
};

pub(crate) fn machine_plugin(app: &mut App) {
    app.add_system_to_stage(StateStage::Transition, transition);
}

#[derive(Debug)]
struct Transition {
    marked: bool,
    trigger: Box<dyn Reflect>,
    state: Box<dyn StateTransition>,
}

impl Clone for Transition {
    fn clone(&self) -> Self {
        Self {
            marked: self.marked,
            trigger: self.trigger.clone_value(),
            state: StateTransition::dyn_clone(&*self.state),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Transitions {
    transitions: Vec<Vec<Transition>>,
    trigger_indices: HashMap<TypeId, usize>,
}

impl Transitions {
    fn add(&mut self, trigger: impl Trigger, state: impl MachineState) {
        let type_id = trigger.type_id();
        let transition = Transition {
            marked: false,
            trigger: trigger.as_reflect().clone_value(),
            state: StateTransition::dyn_clone(&state),
        };

        if let Some(index) = self.trigger_indices.get(&type_id) {
            self.transitions[*index].push(transition);
        } else {
            self.trigger_indices.insert(type_id, self.transitions.len());
            self.transitions.push(vec![transition])
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
    current: Option<Box<dyn StateTransition>>,
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
                .map(|current| StateTransition::dyn_clone(&**current)),
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
        transitions.add(AlwaysTrigger, initial);

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
        self.states
            .entry(TypeId::of::<S>())
            .or_default()
            .transitions
            .add(trigger, state);

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

    pub(crate) fn get_triggers<T: Trigger>(&self) -> impl IntoIterator<Item = T> {
        self.transitions
            .trigger_indices
            .get(&TypeId::of::<T>())
            .map(|index| {
                self.transitions.transitions[*index]
                    .iter()
                    .map(|transition| T::from_reflect(&*transition.trigger).unwrap())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub(crate) fn mark_trigger<T: Trigger>(&mut self, index: usize) {
        self.transitions.transitions[self.transitions.trigger_indices[&TypeId::of::<T>()]][index]
            .marked = true;
    }
}

fn transition(mut commands: Commands, mut machines: Query<(Entity, &mut StateMachine)>) {
    for (entity, mut machine) in &mut machines {
        let Some(state) = machine
            .transitions
            .transitions
            .iter()
            .flatten()
            .find_map(|transition| transition.marked.then_some(&transition.state))
        else { continue };

        let mut entity = commands.entity(entity);

        if let Some(current) = &machine.current {
            current.remove(&mut entity);
        }

        for remove in &machine.removes {
            remove.remove(&mut entity);
        }

        state.insert(&mut entity);
        let metadata = &machine.states[&state.type_id()];

        for insert in &metadata.inserts {
            insert.insert(&mut entity);
        }

        let transitions = metadata.transitions.clone();
        let removes = metadata
            .removes
            .iter()
            .map(|remove| remove.dyn_clone())
            .collect();

        machine.current = Some(StateTransition::dyn_clone(&**state));
        machine.transitions = transitions;
        machine.removes = removes;
    }
}
