// In this game, you control the player by clicking where they should go
// This is an example of how to use `DoneTrigger` and the `Done` component
// `chase.rs` is a better example to start with
// Admittedly, this example exposes some current limitations of this crate, which I will point out

use bevy::{
    ecs::system::{lifetimeless::SRes, StaticSystemParam},
    prelude::*,
    reflect::FromReflect,
};
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(StateMachinePlugin)
        .add_plugin(TriggerPlugin::<Click>::default())
        .init_resource::<CursorPosition>()
        .add_startup_system(init)
        .add_system(update_cursor_position)
        .add_system(find_target.after(update_cursor_position))
        .add_system(go_to_target.after(find_target))
        .run();
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    let go_to_selection = GoToSelection::new(200.);

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("player.png"),
            ..default()
        })
        .insert(Player)
        .insert(
            StateMachine::new(Idle)
                // When the player clicks, go there
                .trans::<Idle>(Click, go_to_selection)
                .trans::<GoToSelection>(Click, go_to_selection)
                // `DoneTrigger` triggers when the `Done` component is added to the entity
                // When they're done going to the selection, idle
                .trans::<GoToSelection>(DoneTrigger, Idle),
        );
}

#[derive(FromReflect, Reflect)]
struct Click;

impl Trigger for Click {
    type Param = (SRes<Input<MouseButton>>, SRes<CursorPosition>);

    fn trigger(&self, _: Entity, param: &StaticSystemParam<Self::Param>) -> bool {
        let (mouse, cursor_position) = &**param;

        mouse.just_pressed(MouseButton::Left) && cursor_position.is_some()
    }
}

#[derive(Clone, Component, Reflect)]
#[component(storage = "SparseSet")]
struct Idle;

#[derive(Clone, Copy, Component, Reflect)]
#[component(storage = "SparseSet")]
struct GoToSelection {
    speed: f32,
    // Limitation: since the state must be created when spawning the state machine, we don't know
    // the target at the time of creating it. So, we must make the target field optional,
    // or add the target as a separate component. I've done the former.
    target: Option<Vec3>,
}

impl GoToSelection {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            target: None,
        }
    }
}

fn find_target(
    mut go_to_selections: Query<&mut GoToSelection, Added<GoToSelection>>,
    cursor_position: Res<CursorPosition>,
) {
    for mut go_to_selection in &mut go_to_selections {
        // Limitation: since the target is computed the frame after clicking, the target
        // may be inaccurate
        go_to_selection.target = Some(cursor_position.unwrap().extend(0.));
    }
}

fn go_to_target(
    mut commands: Commands,
    mut go_to_selections: Query<(Entity, &mut Transform, &GoToSelection)>,
    time: Res<Time>,
) {
    for (entity, mut transform, go_to_selection) in &mut go_to_selections {
        let target = go_to_selection.target.unwrap();
        let delta = target - transform.translation;
        let movement = delta.normalize_or_zero() * go_to_selection.speed * time.delta_seconds();

        if movement.length() > delta.length() {
            transform.translation = target;
            // The player has reached the target!
            // Add the `Done` component to the player, causing `DoneTrigger` to trigger
            // It will be automatically removed later this frame
            commands.entity(entity).insert(Done);
            info!("Done!")
        } else {
            transform.translation += movement;
        }
    }
}

// The code after this comment is not related to `seldom_state`. Just player stuff
// and finding the cursor's world position.

#[derive(Component)]
struct Player;

#[derive(Default, Deref, DerefMut)]
struct CursorPosition(Option<Vec2>);

fn update_cursor_position(
    cameras: Query<&Transform, With<Camera2d>>,
    windows: Res<Windows>,
    mut position: ResMut<CursorPosition>,
) {
    if let Ok(transform) = cameras.get_single() {
        let window = windows.get_primary().unwrap();
        **position = window.cursor_position().map(|cursor_position| {
            (transform.compute_matrix()
                * (cursor_position - Vec2::new(window.width(), window.height()) / 2.)
                    .extend(0.)
                    .extend(1.))
            .truncate()
            .truncate()
        });
    }
}
