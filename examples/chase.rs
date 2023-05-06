// In this game, the player moves around in 2D with the arrow keys, but if they get too close
// to the enemy, the enemy moves towards them, until the player moves back out of range

use bevy::prelude::*;
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // This plugin is required for `seldom_state`
        .add_plugin(StateMachinePlugin)
        .add_startup_system(init)
        .add_system(follow)
        .add_system(move_player)
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

    // Since we use this trigger twice, let's declare it out here so we can reuse it
    let near_player = Near {
        target: player,
        range: 300.,
    };

    // The enemy
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(500., 0., 0.),
            texture: asset_server.load("enemy.png"),
            ..default()
        },
        // This state machine handles the enemy's transitions
        // The initial state is `Idle`
        StateMachine::new(Idle)
            // Add a transition
            // When they're in `Idle` state, and the `near_player` trigger occurs,
            // switch to that instance of the `Follow` state
            .trans::<Idle>(
                near_player,
                // Transitions accept specific instances of states
                Follow {
                    target: player,
                    speed: 100.,
                },
            )
            // Add a second transition
            // When they're in the `Follow` state, and the `near_player` trigger
            // does not occur, switch to the `Idle` state
            // `NotTrigger` is a built-in trigger that negates the given trigger
            .trans::<Follow>(NotTrigger(near_player), Idle)
            // Enable transition logging
            .set_trans_logging(true),
    ));
}

// Let's define our trigger!
// Triggers must implement `Reflect`
// `Clone` and `Copy` are not necessary, but it's nicer to do so here

// This trigger checks if the entity is within the the given range of the target
#[derive(Clone, Copy, Reflect)]
struct Near {
    target: Entity,
    range: f32,
}

// Also see `OptionTrigger` and `BoolTrigger`
impl Trigger for Near {
    // Put the parameters that your trigger needs here
    // Triggers are immutable; you may not access system params mutably
    // Do not query for the `StateMachine` component in this type. This, unfortunately, will panic.
    // `Time` is included here to demonstrate how to get multiple system params
    type Param<'w, 's> = (Query<'w, 's, &'static Transform>, Res<'w, Time>);
    // These types are used by transition builders, for dataflow from triggers to transitions
    // See `StateMachine::trans_builder`
    type Ok = f32;
    type Err = f32;

    // This function checks if the given entity should trigger
    // It runs once per frame for each entity that is in a state that can transition
    // on this trigger
    // Return `Ok` to trigger and `Err` to not trigger
    fn trigger(
        &self,
        entity: Entity,
        (transforms, _time): &Self::Param<'_, '_>,
    ) -> Result<f32, f32> {
        // Find the distance between the target and this entity
        let distance = transforms
            .get(self.target)
            .unwrap()
            .translation
            .truncate()
            .distance(transforms.get(entity).unwrap().translation.truncate());

        // Check whether the target is within range
        // If it is, return `Ok` to trigger!
        (distance <= self.range).then_some(distance).ok_or(distance)
    }
}

// Now let's define our states!
// States must implement `Bundle`, `Clone`, and `Reflect`
// `MachineState` is implemented automatically for valid states
// If necessary, you may use `#[reflect(ignore)]` on fields that cannot be reflected
// Consider annotating your states with `#[component(storage = "SparseSet")]`
// for better insert/remove performance
// Don't insert/remove states manually! This will confuse the `StateMachine`.
// You may mutate states, though.

// Entities in the `Idle` state should do nothing
#[derive(Clone, Component, Reflect)]
#[component(storage = "SparseSet")]
struct Idle;

// Entities in the `Follow` state should move towards the given entity at the given speed
#[derive(Clone, Component, Reflect)]
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

// The code after this comment is not related to `seldom_state`. It's just player movement.

#[derive(Component)]
struct Player;

const PLAYER_SPEED: f32 = 200.;

fn move_player(
    mut players: Query<&mut Transform, With<Player>>,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    players.single_mut().translation += Vec3::new(
        (keys.pressed(KeyCode::Right) as i32 - keys.pressed(KeyCode::Left) as i32) as f32,
        (keys.pressed(KeyCode::Up) as i32 - keys.pressed(KeyCode::Down) as i32) as f32,
        0.,
    )
    .normalize_or_zero()
        * PLAYER_SPEED
        * time.delta_seconds();
}
