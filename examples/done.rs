// In this game, you control the player by clicking where they should go. This is an example of how
// to use `DoneTrigger` and the `Done` component. `chase.rs` is a better example to start with.

use bevy::prelude::*;
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, StateMachinePlugin))
        .init_resource::<CursorPosition>()
        .add_systems(Startup, init)
        .add_systems(Update, (update_cursor_position, go_to_target))
        .run();
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Player,
        Idle,
        StateMachine::default()
            // When the player clicks, go there
            .trans_builder(click, |_: &AnyState, pos| {
                Some(GoToSelection {
                    speed: 200.,
                    target: pos,
                })
            })
            // `done` triggers when the `Done` component is added to the entity. When they're done
            // going to the selection, idle.
            .trans::<GoToSelection, _>(done(Some(Done::Success)), Idle)
            .set_trans_logging(true),
        Sprite::from_image(asset_server.load("player.png")),
    ));
}

fn click(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_position: Res<CursorPosition>,
) -> Option<Vec2> {
    mouse
        .just_pressed(MouseButton::Left)
        .then_some(())
        .and(**cursor_position)
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
            // The player has reached the target! Add the `Done` component to the player, causing
            // `done` to trigger. It will be automatically removed later this frame.
            commands.entity(entity).insert(Done::Success);
            info!("Done!")
        } else {
            transform.translation += movement.extend(0.);
        }
    }
}

// The code after this comment is not related to `seldom_state`. Just player stuff and finding the
// cursor's world position.

#[derive(Component)]
struct Player;

#[derive(Default, Deref, DerefMut, Resource)]
struct CursorPosition(Option<Vec2>);

fn update_cursor_position(
    cameras: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut position: ResMut<CursorPosition>,
) {
    let (camera, transform) = cameras.single();
    **position = windows
        .single()
        .cursor_position()
        .and_then(|cursor_position| camera.viewport_to_world_2d(transform, cursor_position));
}
