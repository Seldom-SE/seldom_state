use std::any::{type_name, TypeId};

use bevy::utils::HashMap;

use crate::{prelude::*, stage::StateStage, state::StateTransition};

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
            state: self.state.dyn_clone(),
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
            state: state.dyn_clone(),
        });

        Ok(())
    }
}

/// State machine component. Entities with this component will have components (the states)
/// added and removed based on the transitions that you add. Build one with [`StateMachine::new()`]
/// and [`StateMachine::trans()`].
#[derive(Component, Debug)]
pub struct StateMachine {
    current: Option<Box<dyn StateTransition>>,
    transitions: Transitions,
    states: HashMap<TypeId, Transitions>,
}

impl Clone for StateMachine {
    fn clone(&self) -> Self {
        Self {
            current: self.current.as_ref().map(|current| current.dyn_clone()),
            transitions: self.transitions.clone(),
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
    /// StateMachine::new(Idle)
    ///     .trans::<Idle>(PressA, Jump)
    ///     .trans::<Idle>(PressA, SwingSword); // Panic!
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

            state.insert(&mut entity);
            let transitions = machine.states[&state.type_id()].clone();
            machine.current = Some(state.dyn_clone());
            machine.transitions = transitions;
        }
    }
}
