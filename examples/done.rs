// In this game, you control the player by clicking where they should go
// This is an example of how to use `DoneTrigger` and the `Done` component
// `chase.rs` is a better example to start with

use bevy::prelude::*;
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(StateMachinePlugin)
        .init_resource::<CursorPosition>()
        .add_startup_system(init)
        .add_system(update_cursor_position)
        .add_system(go_to_target)
        .run();
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("player.png"),
            ..default()
        },
        Player,
        StateMachine::default()
            // When the player clicks, go there
            .trans_builder::<AnyState, _, _>(Click, |pos| {
                Some(GoToSelection {
                    speed: 200.,
                    target: pos,
                })
            })
            // `DoneTrigger` triggers when the `Done` component is added to the entity
            // When they're done going to the selection, idle
            .trans::<GoToSelection>(DoneTrigger::Success, Idle)
            .set_trans_logging(true),
        Idle,
    ));
}

#[derive(Clone, Reflect)]
struct Click;

impl OptionTrigger for Click {
    type Param<'w, 's> = (Res<'w, Input<MouseButton>>, Res<'w, CursorPosition>);
    type Some = Vec2;

    fn trigger(&self, _: Entity, (mouse, cursor_position): &Self::Param<'_, '_>) -> Option<Vec2> {
        mouse
            .just_pressed(MouseButton::Left)
            .then_some(())
            .and(***cursor_position)
    }
}

#[derive(Clone, Component, Reflect)]
#[component(storage = "SparseSet")]
struct Idle;

#[derive(Clone, Copy, Component, Reflect)]
#[component(storage = "SparseSet")]
struct GoToSelection {
    speed: f32,
    target: Vec2,
}

fn go_to_target(
    mut commands: Commands,
    mut go_to_selections: Query<(Entity, &mut Transform, &GoToSelection)>,
    time: Res<Time>,
) {
    for (entity, mut transform, go_to_selection) in &mut go_to_selections {
        let target = go_to_selection.target;
        let delta = target - transform.translation.truncate();
        let movement = delta.normalize_or_zero() * go_to_selection.speed * time.delta_seconds();

        if movement.length() > delta.length() {
            transform.translation = target.extend(transform.translation.z);
            // The player has reached the target!
            // Add the `Done` component to the player, causing `DoneTrigger` to trigger
            // It will be automatically removed later this frame
            commands.entity(entity).insert(Done::Success);
            info!("Done!")
        } else {
            transform.translation += movement.extend(0.);
        }
    }
}

// The code after this comment is not related to `seldom_state`. Just player stuff
// and finding the cursor's world position.

#[derive(Component)]
struct Player;

#[derive(Default, Deref, DerefMut, Resource)]
struct CursorPosition(Option<Vec2>);

fn update_cursor_position(
    cameras: Query<&Transform, With<Camera2d>>,
    windows: Query<&Window>,
    mut position: ResMut<CursorPosition>,
) {
    let window = windows.single();
    **position = window.cursor_position().map(|cursor_position| {
        (cameras.single().compute_matrix()
            * (cursor_position - Vec2::new(window.width(), window.height()) / 2.)
                .extend(0.)
                .extend(1.))
        .truncate()
        .truncate()
    });
}
