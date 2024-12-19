use std::{any::type_name, ops::Range};

use bevy::math::InvalidDirectionError;
use leafwing_input_manager::action_state::ActionData;

use crate::prelude::*;

/// Trigger that transitions if the given [`Actionlike`]'s value is within the given bounds.
/// Consider using `f32::NEG_INFINITY`/`f32::INFINITY` in the bounds.
pub fn value<A: Actionlike>(
    action: A,
    bounds: Range<f32>,
) -> impl EntityTrigger<Out = Result<f32, f32>> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        let value = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ValueTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .value(&action);

        if bounds.contains(&value) {
            Ok(value)
        } else {
            Err(value)
        }
    })
    .into_trigger()
}

/// Unbounded [`value`]
pub fn value_unbounded(action: impl Actionlike) -> impl EntityTrigger<Out = Result<f32, f32>> {
    value(action, f32::NEG_INFINITY..f32::INFINITY)
}

/// [`value`] with only a minimum bound
pub fn value_min(action: impl Actionlike, min: f32) -> impl EntityTrigger<Out = Result<f32, f32>> {
    value(action, min..f32::INFINITY)
}

/// [`value`] with only a maximum bound
pub fn value_max(action: impl Actionlike, max: f32) -> impl EntityTrigger<Out = Result<f32, f32>> {
    value(action, f32::NEG_INFINITY..max)
}

/// [`value`] clamped to [-1, 1]
pub fn clamped_value<A: Actionlike>(
    action: A,
    bounds: Range<f32>,
) -> impl EntityTrigger<Out = Result<f32, f32>> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        let value = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ClampedValueTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .clamped_value(&action);

        if bounds.contains(&value) {
            Ok(value)
        } else {
            Err(value)
        }
    })
    .into_trigger()
}

/// Unbounded [`clamped_value`]
pub fn clamped_value_unbounded(
    action: impl Actionlike,
) -> impl EntityTrigger<Out = Result<f32, f32>> {
    clamped_value(action, f32::NEG_INFINITY..f32::INFINITY)
}

/// [`clamped_value`] with only a minimum bound
pub fn clamped_value_min(
    action: impl Actionlike,
    min: f32,
) -> impl EntityTrigger<Out = Result<f32, f32>> {
    clamped_value(action, min..f32::INFINITY)
}

/// [`clamped_value`] with only a maximum bound
pub fn clamped_value_max(
    action: impl Actionlike,
    max: f32,
) -> impl EntityTrigger<Out = Result<f32, f32>> {
    clamped_value(action, f32::NEG_INFINITY..max)
}

/// Trigger that transitions if the given [`Actionlike`]'s [`DualAxisData`] is within the given
/// bounds. If no minimum length is necessary, use `0.`. To exclude specifically neutral axis pairs,
/// use a small positive value. If no maximum length is necessary, use `f32::INFINITY`, or similar.
/// If rotation bounds are not necessary, use the same value for the minimum and maximum ex.
/// `Dir2::Y..Dir2::Y`.
pub fn axis_pair<A: Actionlike>(
    action: A,
    length_bounds: Range<f32>,
    rotation_bounds: Range<Dir2>,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        let axis_pair = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `AxisPairTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .axis_pair(&action);

        match Dir2::new_and_length(axis_pair) {
            Ok((rotation, length)) => {
                length_bounds.contains(&length)
                    && (rotation_bounds.start == rotation_bounds.end
                        || ({
                            let start = rotation_bounds.start.to_angle();
                            let end = rotation_bounds.end.to_angle();
                            let rotation = rotation.to_angle();

                            if start < end {
                                rotation >= start && rotation <= end
                            } else {
                                rotation >= start || rotation <= end
                            }
                        }))
            }
            Err(InvalidDirectionError::Zero) => length_bounds.contains(&0.),
            Err(InvalidDirectionError::Infinite | InvalidDirectionError::NaN) => false,
        }
        .then_some(axis_pair)
        .ok_or(axis_pair)
    })
    .into_trigger()
}

/// Unbounded [`axis_pair`]
pub fn axis_pair_unbounded(
    action: impl Actionlike,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    axis_pair(action, 0.0..f32::INFINITY, Dir2::Y..Dir2::Y)
}

/// [`axis_pair`] with only a minimum length bound
pub fn axis_pair_min_length(
    action: impl Actionlike,
    min_length: f32,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    axis_pair(action, min_length..f32::INFINITY, Dir2::Y..Dir2::Y)
}

/// [`axis_pair`] with only a maximum length bound
pub fn axis_pair_max_length(
    action: impl Actionlike,
    max_length: f32,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    axis_pair(action, 0.0..max_length, Dir2::Y..Dir2::Y)
}

/// [`axis_pair`] with only length bounds
pub fn axis_pair_length_bounds(
    action: impl Actionlike,
    length_bounds: Range<f32>,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    axis_pair(action, length_bounds, Dir2::Y..Dir2::Y)
}

/// [`axis_pair`] with only rotation bounds
pub fn axis_pair_rotation_bounds(
    action: impl Actionlike,
    rotation_bounds: Range<Dir2>,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    axis_pair(action, 0.0..f32::INFINITY, rotation_bounds)
}

/// [`axis_pair`] with axes clamped to [-1, 1]
pub fn clamped_axis_pair<A: Actionlike>(
    action: A,
    length_bounds: Range<f32>,
    rotation_bounds: Range<Dir2>,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        let axis_pair = actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `AxisPairTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .clamped_axis_pair(&action);

        match Dir2::new_and_length(axis_pair) {
            Ok((rotation, length)) => {
                length_bounds.contains(&length)
                    && (rotation_bounds.start == rotation_bounds.end
                        || ({
                            let start = rotation_bounds.start.to_angle();
                            let end = rotation_bounds.end.to_angle();
                            let rotation = rotation.to_angle();

                            if start < end {
                                rotation >= start && rotation <= end
                            } else {
                                rotation >= start || rotation <= end
                            }
                        }))
            }
            Err(InvalidDirectionError::Zero) => length_bounds.contains(&0.),
            Err(InvalidDirectionError::Infinite | InvalidDirectionError::NaN) => false,
        }
        .then_some(axis_pair)
        .ok_or(axis_pair)
    })
    .into_trigger()
}

/// Unbounded [`clamped_axis_pair`]
pub fn clamped_axis_pair_unbounded(
    action: impl Actionlike,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    clamped_axis_pair(action, 0.0..f32::INFINITY, Dir2::Y..Dir2::Y)
}

/// [`clamped_axis_pair`] with only a minimum length bound
pub fn clamped_axis_pair_min_length(
    action: impl Actionlike,
    min_length: f32,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    clamped_axis_pair(action, min_length..f32::INFINITY, Dir2::Y..Dir2::Y)
}

/// [`clamped_axis_pair`] with only a maximum length bound
pub fn clamped_axis_pair_max_length(
    action: impl Actionlike,
    max_length: f32,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    clamped_axis_pair(action, 0.0..max_length, Dir2::Y..Dir2::Y)
}

/// [`clamped_axis_pair`] with only length bounds
pub fn clamped_axis_pair_length_bounds(
    action: impl Actionlike,
    length_bounds: Range<f32>,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    clamped_axis_pair(action, length_bounds, Dir2::Y..Dir2::Y)
}

/// [`clamped_axis_pair`] with only rotation bounds
pub fn clamped_axis_pair_rotation_bounds(
    action: impl Actionlike,
    rotation_bounds: Range<Dir2>,
) -> impl EntityTrigger<Out = Result<Vec2, Vec2>> {
    clamped_axis_pair(action, 0.0..f32::INFINITY, rotation_bounds)
}

/// Trigger that transitions upon pressing the given [`Actionlike`]
pub fn just_pressed<A: Actionlike>(action: A) -> impl EntityTrigger<Out = bool> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `JustPressedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .just_pressed(&action)
    })
    .into_trigger()
}

/// Trigger that transitions while pressing the given [`Actionlike`]
pub fn pressed<A: Actionlike>(action: A) -> impl EntityTrigger<Out = bool> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `JustPressedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .pressed(&action)
    })
    .into_trigger()
}

/// Trigger that transitions upon releasing the given [`Actionlike`]
pub fn just_released<A: Actionlike>(action: A) -> impl EntityTrigger<Out = bool> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `JustPressedTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .just_released(&action)
    })
    .into_trigger()
}

/// Provides the given [`Actionlike`]'s [`ActionData`]
pub fn action_data<A: Actionlike>(action: A) -> impl EntityTrigger<Out = Option<ActionData>> {
    (move |In(entity): In<Entity>, actors: Query<&ActionState<A>>| {
        actors
            .get(entity)
            .unwrap_or_else(|_| {
                panic!(
                    "entity {entity:?} with `ActionDataTrigger<{0}>` is missing `ActionState<{0}>`",
                    type_name::<A>()
                )
            })
            .action_data(&action)
            .cloned()
    })
    .into_trigger()
}
