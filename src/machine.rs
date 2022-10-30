use std::any::{type_name, TypeId};

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
    transitions: Vec<Transition>,
    trigger_indices: HashMap<TypeId, usize>,
}

impl Transitions {
    fn add(&mut self, trigger: impl Trigger, state: impl MachineState) -> Result<(), usize> {
        self.trigger_indices
            .insert(trigger.type_id(), self.transitions.len())
            .map_or_else(|| Ok(()), Err)?;

        self.transitions.push(Transition {
            marked: false,
            trigger: trigger.as_reflect().clone_value(),
            state: StateTransition::dyn_clone(&state),
        });

        Ok(())
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
        transitions.add(AlwaysTrigger, initial).unwrap();

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
    ///
    /// # Panics
    ///
    /// Panics when attempting to add a transition from a state and on a trigger
    /// for which there is already a state to transition to. For example, the following code panics:
    ///
    /// ```should_panic
    /// # use bevy::{
    /// #     ecs::system::StaticSystemParam,
    /// #     prelude::*,
    /// #     reflect::FromReflect,
    /// # };
    /// # use seldom_state::prelude::*;
    /// #
    /// # #[derive(Clone, Component, Reflect)]
    /// # struct Idle;
    /// #
    /// # #[derive(FromReflect, Reflect)]
    /// # struct PressA;
    /// #
    /// # impl Trigger for PressA {
    /// #     type Param = ();
    /// #
    /// #     fn trigger(&self, _: Entity, _: &StaticSystemParam<Self::Param>) -> bool {
    /// #         true
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Component, Reflect)]
    /// # struct Jump;
    /// #
    /// # #[derive(Clone, Component, Reflect)]
    /// # struct SwingSword;
    /// #
    /// StateMachine::new((Idle,))
    ///     .trans::<(Idle,)>(PressA, (Jump,))
    ///     .trans::<(Idle,)>(PressA, (SwingSword,)); // Panic!
    /// ```
    pub fn trans<S: MachineState>(
        mut self,
        trigger: impl Trigger,
        state: impl MachineState,
    ) -> Self {
        let trigger_type = String::from(trigger.type_name());
        let state_type = String::from(state.type_name());

        if let Err(index) = self
            .states
            .entry(TypeId::of::<S>())
            .or_default()
            .transitions
            .add(trigger, state)
        {
            panic!(
                "attempted to add a transition from {} on {} to {} when one already existed to {}",
                type_name::<S>(),
                trigger_type,
                state_type,
                self.transitions.transitions[index].state.type_name()
            )
        }

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

    pub(crate) fn get_trigger<T: Trigger>(&self) -> Option<T> {
        self.transitions
            .trigger_indices
            .get(&TypeId::of::<T>())
            .map(|index| T::from_reflect(&*self.transitions.transitions[*index].trigger).unwrap())
    }

    pub(crate) fn mark_trigger<T: Trigger>(&mut self) {
        self.transitions.transitions[self.transitions.trigger_indices[&TypeId::of::<T>()]].marked =
            true;
    }
}

fn transition(mut commands: Commands, mut machines: Query<(Entity, &mut StateMachine)>) {
    for (entity, mut machine) in &mut machines {
        if let Some(state) = machine
            .transitions
            .transitions
            .iter()
            .find_map(|transition| transition.marked.then_some(&transition.state))
        {
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
}
