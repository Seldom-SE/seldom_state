// In this game, the player moves around in 2D with the arrow keys, but if they get too close to the
// enemy, the enemy moves towards them, until the player moves back out of range

use bevy::prelude::*;
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, StateMachinePlugin))
        // This plugin is required for `seldom_state`
        .add_systems(Startup, init)
        .add_systems(Update, (follow, move_player))
        .run();
}

// Setup the game
fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // Simple player entity
    let player = commands
        .spawn((
            SpriteBundle {
                texture: asset_server.load("player.png"),
                ..default()
            },
            Player,
        ))
        .id();

    // This is our trigger, which is a Bevy system that returns a `bool`, `Option`, or `Result`. We
    // define the trigger as a closure within this function so it can use variables in the scope
    // (namely, `player`). For the sake of example, we also define this trigger as an external
    // function later.
    //
    // Triggers are reinitialized after each transition, so they won't read events that occurred in a
    // previous state, `Local`s are reset between transitions, etc.
    let near_player = move |In(entity): In<Entity>, transforms: Query<&Transform>| {
        let distance = transforms
            .get(player)
            .unwrap()
            .translation
            .truncate()
            .distance(transforms.get(entity).unwrap().translation.truncate());

        // Check whether the target is within range. If it is, return `Ok` to trigger!
        match distance <= 300. {
            true => Ok(distance),
            false => Err(distance),
        }
    };

    // The enemy
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(500., 0., 0.),
            texture: asset_server.load("enemy.png"),
            ..default()
        },
        // This state machine handles the enemy's transitions. Transitions defined earlier have
        // priority, but triggers after the first accepted one may still be checked.
        StateMachine::default()
            // Add a transition. When they're in `Idle` state, and the `near_player` trigger occurs,
            // switch to this instance of the `Follow` state
            .trans::<Idle, _>(
                near_player,
                // Transitions accept specific instances of states
                Follow {
                    target: player,
                    speed: 100.,
                },
            )
            // Add a second transition. When they're in the `Follow` state, and the `near_player`
            // trigger does not occur, switch to the `Idle` state. `.not()` is a combinator that
            // negates the trigger. `.and(other)` and `.or(other)` also exist.
            .trans::<Follow, _>(near_player.not(), Idle)
            // Enable transition logging
            .set_trans_logging(true),
        // The initial state is `Idle`
        Idle,
    ));
}

// Now let's define our states! States must implement `Component` and `Clone`. `EntityState` is
// implemented automatically for valid states. Feel free to mutate/insert/remove states manually,
// but don't put your entity in multiple or zero states, else it will panic. Manually inserted/
// removed states will not trigger `on_enter`/`on_exit` events registered to the `StateMachine`.

// Entities in the `Idle` state do nothing
#[derive(Clone, Component)]
#[component(storage = "SparseSet")]
struct Idle;

// Entities in the `Follow` state move toward the given entity at the given speed
#[derive(Clone, Component)]
#[component(storage = "SparseSet")]
struct Follow {
    target: Entity,
    speed: f32,
}

// Let's define some behavior for entities in the follow state
fn follow(
    mut transforms: Query<&mut Transform>,
    follows: Query<(Entity, &Follow)>,
    time: Res<Time>,
) {
    for (entity, follow) in &follows {
        // Get the positions of the follower and target
        let target_translation = transforms.get(follow.target).unwrap().translation;
        let follow_transform = &mut transforms.get_mut(entity).unwrap();
        let follow_translation = follow_transform.translation;

        // Find the direction from the follower to the target and go that way
        follow_transform.translation += (target_translation - follow_translation)
            .normalize_or_zero()
            * follow.speed
            * time.delta_seconds();
    }
}

// For the sake of example, this is a function that returns the `near_player` trigger from before.
// This may be useful so that triggers that accept case-by-case values may be used across the
// codebase. Triggers that don't need to accept any values from local code may be defined as normal
// Bevy systems (see the `done` example). Also consider implementing the `Trigger` trait directly.
#[allow(dead_code)]
fn near(target: Entity) -> impl EntityTrigger<Out = Result<f32, f32>> {
    (move |In(entity): In<Entity>, transforms: Query<&Transform>| {
        let distance = transforms
            .get(target)
            .unwrap()
            .translation
            .truncate()
            .distance(transforms.get(entity).unwrap().translation.truncate());

        // Check whether the target is within range. If it is, return `Ok` to trigger!
        match distance <= 300. {
            true => Ok(distance),
            false => Err(distance),
        }
    })
    .into_trigger()
}

// The code after this comment is not related to `seldom_state`. It's just player movement.

#[derive(Component)]
struct Player;

const PLAYER_SPEED: f32 = 200.;

fn move_player(
    mut players: Query<&mut Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    players.single_mut().translation += Vec3::new(
        (keys.pressed(KeyCode::ArrowRight) as i32 - keys.pressed(KeyCode::ArrowLeft) as i32) as f32,
        (keys.pressed(KeyCode::ArrowUp) as i32 - keys.pressed(KeyCode::ArrowDown) as i32) as f32,
        0.,
    )
    .normalize_or_zero()
        * PLAYER_SPEED
        * time.delta_seconds();
}
