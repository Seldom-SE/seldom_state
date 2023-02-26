use std::{any::type_name, marker::PhantomData};

use leafwing_input_manager::{
    action_state::ActionData, axislike::DualAxisData, orientation::Rotation,
};

use crate::prelude::*;

/// Plugin that registers input-related trigger for the given [`Actionlike`]
pub struct InputTriggerPlugin<A: Actionlike + Reflect>(PhantomData<A>);

impl<A: Actionlike + Reflect> Plugin for InputTriggerPlugin<A> {
    fn build(&self, app: &mut App) {
        app.fn_plugin(input_trigger_plugin::<A>);
    }
}

impl<A: Actionlike + Reflect> Default for InputTriggerPlugin<A> {
    fn default() -> Self {
        Self(default())
    }
}

/// Function called by [`InputTriggerPlugin`]. You may instead call it directly
/// or use `seldom_fn_plugin`, which is another crate I maintain.
pub fn input_trigger_plugin<A: Actionlike + Reflect>(app: &mut App) {
    app.fn_plugin(trigger_plugin::<ActionDataTrigger<A>>)
        .fn_plugin(trigger_plugin::<AxisPairTrigger<A>>)
        .fn_plugin(trigger_plugin::<ClampedAxisPairTrigger<A>>)
        .fn_plugin(trigger_plugin::<ClampedValueTrigger<A>>)
        .fn_plugin(trigger_plugin::<JustPressedTrigger<A>>)
        .fn_plugin(trigger_plugin::<JustReleasedTrigger<A>>)
        .fn_plugin(trigger_plugin::<PressedTrigger<A>>)
        .fn_plugin(trigger_plugin::<ReleasedTrigger<A>>)
        .fn_plugin(trigger_plugin::<ValueTrigger<A>>);
}

/// Trigger that transitions if the given [`Actionlike`]'s value is within the given bounds.
/// You must add an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Reflect)]
pub struct ValueTrigger<A: Actionlike + Reflect> {
    /// The action
    pub action: A,
    /// The minimum value. If no minimum is necessary, use [`f32::NEG_INFINITY`], or similar
    pub min: f32,
    /// The maximum value. If no maximum is necessary, use [`f32::INFINITY`], or similar
    pub max: f32,
}

impl<A: Actionlike + Reflect> Trigger for ValueTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;
    type Ok = f32;
    type Err = f32;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> Result<f32, f32> {
        let value = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ValueTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .value(self.action.clone());

        (value >= self.min && value <= self.max)
            .then_some(value)
            .ok_or(value)
    }
}

impl<A: Actionlike + Reflect> ValueTrigger<A> {
    /// Unbounded trigger
    pub fn unbounded(action: A) -> Self {
        Self {
            action,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }

    /// Trigger with a minimum bound
    pub fn min(action: A, min: f32) -> Self {
        Self {
            action,
            min,
            max: f32::INFINITY,
        }
    }

    /// Trigger with a maximum bound
    pub fn max(action: A, max: f32) -> Self {
        Self {
            action,
            min: f32::NEG_INFINITY,
            max,
        }
    }
}

/// Trigger that transitions if the given [`Actionlike`]'s value is within the given bounds.
/// You must add an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Reflect)]
pub struct ClampedValueTrigger<A: Actionlike + Reflect> {
    /// The action
    pub action: A,
    /// The minimum value. If no minimum is necessary, use `f32::NEG_INFINITY`, or similar.
    pub min: f32,
    /// The maximum value. If no maximum is necessary, use `f32::INFINITY`, or similar.
    pub max: f32,
}

impl<A: Actionlike + Reflect> Trigger for ClampedValueTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;
    type Ok = f32;
    type Err = f32;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> Result<f32, f32> {
        let value = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ClampedValueTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .clamped_value(self.action.clone());

        (value >= self.min && value <= self.max)
            .then_some(value)
            .ok_or(value)
    }
}

impl<A: Actionlike + Reflect> ClampedValueTrigger<A> {
    /// Unbounded trigger
    pub fn unbounded(action: A) -> Self {
        Self {
            action,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }

    /// Trigger with a minimum bound
    pub fn min(action: A, min: f32) -> Self {
        Self {
            action,
            min,
            max: f32::INFINITY,
        }
    }

    /// Trigger with a maximum bound
    pub fn max(action: A, max: f32) -> Self {
        Self {
            action,
            min: f32::NEG_INFINITY,
            max,
        }
    }
}

/// Trigger that transitions if the given [`Actionlike`]'s [`DualAxisData`] is within the given
/// bounds. You must add an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Reflect)]
pub struct AxisPairTrigger<A: Actionlike + Reflect> {
    /// The action
    pub action: A,
    /// Minimum axis pair length. If no minimum is necessary, use `0.`. To exclude specifically
    /// neutral axis pairs, use `f32::EPSILON`, or similar.
    pub min_length: f32,
    /// Maximum axis pair length. If no maximum is necessary, use `f32::INFINITY`, or similar.
    pub max_length: f32,
    /// Minimum rotation, measured clockwise from midnight. If rotation bounds are not necessary,
    /// set this and `max_rotation` to the same value.
    #[reflect(ignore)]
    pub min_rotation: Rotation,
    /// Maximum rotation, measured clockwise from midnight. If rotation bounds are not necessary,
    /// set this and `min_rotation` to the same value.
    #[reflect(ignore)]
    pub max_rotation: Rotation,
}

impl<A: Actionlike + Reflect> Trigger for AxisPairTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;
    type Ok = DualAxisData;
    type Err = Option<DualAxisData>;

    fn trigger(
        &self,
        entity: Entity,
        actors: &Self::Param<'_, '_>,
    ) -> Result<DualAxisData, Option<DualAxisData>> {
        let axis_pair = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `AxisPairTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .axis_pair(self.action.clone());

        axis_pair
            .and_then(|axis_pair| {
                let length = axis_pair.length();
                let rotation = axis_pair.rotation();

                (length >= self.min_length
                    && length <= self.max_length
                    && rotation
                        .map(|rotation| match self.min_rotation <= self.max_rotation {
                            true => rotation >= self.min_rotation && rotation <= self.max_rotation,
                            false => rotation >= self.min_rotation || rotation <= self.max_rotation,
                        })
                        .unwrap_or(true))
                .then_some(axis_pair)
            })
            .ok_or(axis_pair)
    }
}

impl<A: Actionlike + Reflect> AxisPairTrigger<A> {
    /// Unbounded trigger
    pub fn unbounded(action: A) -> Self {
        Self {
            action,
            min_length: 0.,
            max_length: f32::INFINITY,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with a minimum length bound
    pub fn min_length(action: A, min_length: f32) -> Self {
        Self {
            action,
            min_length,
            max_length: f32::INFINITY,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with a maximum length bound
    pub fn max_length(action: A, max_length: f32) -> Self {
        Self {
            action,
            min_length: 0.,
            max_length,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with length bounds
    pub fn length_bounds(action: A, min_length: f32, max_length: f32) -> Self {
        Self {
            action,
            min_length,
            max_length,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with rotation bounds
    pub fn rotation_bounds(action: A, min_rotation: Rotation, max_rotation: Rotation) -> Self {
        Self {
            action,
            min_length: 0.,
            max_length: f32::INFINITY,
            min_rotation,
            max_rotation,
        }
    }
}

/// Trigger that transitions if the given [`Actionlike`]'s [`DualAxisData`] is within the given
/// bounds. You must add an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Reflect)]
pub struct ClampedAxisPairTrigger<A: Actionlike + Reflect> {
    /// The action
    pub action: A,
    /// Minimum axis pair length. If no minimum is necessary, use `0.`. To exclude specifically
    /// neutral axis pairs, use `f32::EPSILON`, or similar.
    pub min_length: f32,
    /// Maximum axis pair length. If no maximum is necessary, use `f32::INFINITY`, or similar.
    pub max_length: f32,
    /// Minimum rotation, measured clockwise from midnight. If rotation bounds are not necessary,
    /// set this and `max_rotation` to the same value.
    #[reflect(ignore)]
    pub min_rotation: Rotation,
    /// Maximum rotation, measured clockwise from midnight. If rotation bounds are not necessary,
    /// set this and `min_rotation` to the same value.
    #[reflect(ignore)]
    pub max_rotation: Rotation,
}

impl<A: Actionlike + Reflect> Trigger for ClampedAxisPairTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;
    type Ok = DualAxisData;
    type Err = Option<DualAxisData>;

    fn trigger(
        &self,
        entity: Entity,
        actors: &Self::Param<'_, '_>,
    ) -> Result<DualAxisData, Option<DualAxisData>> {
        let axis_pair = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ClampedAxisPairTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .axis_pair(self.action.clone());

        axis_pair
            .and_then(|axis_pair| {
                let length = axis_pair.length();
                let rotation = axis_pair.rotation();

                (length >= self.min_length
                    && length <= self.max_length
                    && rotation
                        .map(|rotation| match self.min_rotation <= self.max_rotation {
                            true => rotation >= self.min_rotation && rotation <= self.max_rotation,
                            false => rotation >= self.min_rotation || rotation <= self.max_rotation,
                        })
                        .unwrap_or(true))
                .then_some(axis_pair)
            })
            .ok_or(axis_pair)
    }
}

impl<A: Actionlike + Reflect> ClampedAxisPairTrigger<A> {
    /// Unbounded trigger
    pub fn unbounded(action: A) -> Self {
        Self {
            action,
            min_length: 0.,
            max_length: f32::INFINITY,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with a minimum length bound
    pub fn min_length(action: A, min_length: f32) -> Self {
        Self {
            action,
            min_length,
            max_length: f32::INFINITY,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with a maximum length bound
    pub fn max_length(action: A, max_length: f32) -> Self {
        Self {
            action,
            min_length: 0.,
            max_length,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with length bounds
    pub fn length_bounds(action: A, min_length: f32, max_length: f32) -> Self {
        Self {
            action,
            min_length,
            max_length,
            min_rotation: Rotation::NORTH,
            max_rotation: Rotation::NORTH,
        }
    }

    /// Trigger with rotation bounds
    pub fn rotation_bounds(action: A, min_rotation: Rotation, max_rotation: Rotation) -> Self {
        Self {
            action,
            min_length: 0.,
            max_length: f32::INFINITY,
            min_rotation,
            max_rotation,
        }
    }
}

/// Trigger that transitions upon pressing the given [`Actionlike`]. You must add
/// an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct JustPressedTrigger<A: Actionlike + Reflect>(pub A);

impl<A: Actionlike + Reflect> BoolTrigger for JustPressedTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> bool {
        let Self(action) = self;
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `JustPressedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .just_pressed(action.clone())
    }
}

/// Trigger that transitions while pressing the given [`Actionlike`]. You must add
/// an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct PressedTrigger<A: Actionlike + Reflect>(pub A);

impl<A: Actionlike + Reflect> BoolTrigger for PressedTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> bool {
        let Self(action) = self;
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `PressedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .just_pressed(action.clone())
    }
}

/// Trigger that transitions upon releasing the given [`Actionlike`]. You must add
/// an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct JustReleasedTrigger<A: Actionlike + Reflect>(pub A);

#[cfg(feature = "leafwing_input")]
impl<A: Actionlike + Reflect> BoolTrigger for JustReleasedTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> bool {
        let Self(action) = self;
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `JustReleasedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .just_released(action.clone())
    }
}

/// Trigger that transitions while the given [`Actionlike`] is released. You must add
/// an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct ReleasedTrigger<A: Actionlike + Reflect>(pub A);

impl<A: Actionlike + Reflect> BoolTrigger for ReleasedTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> bool {
        let Self(action) = self;
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ReleasedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .just_pressed(action.clone())
    }
}

/// Trigger that always transitions, providing the given [`Actionlike`]'s [`ActionData`].
/// You must add an [`InputTriggerPlugin<Actionlike>`] with your [`Actionlike`] type.
#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct ActionDataTrigger<A: Actionlike + Reflect>(pub A);

impl<A: Actionlike + Reflect> Trigger for ActionDataTrigger<A> {
    type Param<'w, 's> = Query<'w, 's, &'static ActionState<A>>;
    type Ok = ActionData;
    type Err = Never;

    fn trigger(&self, entity: Entity, actors: &Self::Param<'_, '_>) -> Result<ActionData, Never> {
        let Self(action) = self;
        Ok(actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ActionDataTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .action_data(action.clone())
            .clone())
    }
}
