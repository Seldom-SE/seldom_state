// In this game, you can move with the left and right arrow keys, and jump with space
// `leafwing-input-manager` handles the input

use bevy::prelude::*;
use leafwing_input_manager::{axislike::VirtualAxis, prelude::*};
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_plugin(StateMachinePlugin)
        .add_plugin(InputTriggerPlugin::<Action>::default())
        .add_plugin(TriggerPlugin::<GroundedTrigger>::default())
        .add_startup_system(init)
        .add_system(walk)
        .add_system(fall)
        .run();
}

const JUMP_VELOCITY: f32 = 500.;

// Setup the game
fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(500., 0., 0.),
            texture: asset_server.load("player.png"),
            ..default()
        },
        InputManagerBundle {
            input_map: InputMap::default()
                .insert(VirtualAxis::horizontal_arrow_keys(), Action::Move)
                .insert(
                    SingleAxis::symmetric(GamepadAxisType::LeftStickX, 0.),
                    Action::Move,
                )
                .insert(KeyCode::Space, Action::Jump)
                .insert(GamepadButtonType::South, Action::Jump)
                .build(),
            ..default()
        },
        StateMachine::new(Grounded::Idle)
            .trans::<Grounded>(
                JustPressedTrigger(Action::Jump),
                Falling {
                    velocity: JUMP_VELOCITY,
                },
            )
            .trans_builder(GroundedTrigger, |falling: &Falling, _| {
                (dbg!(falling.velocity) <= 0.).then_some(Grounded::Idle)
            })
            .trans_builder(
                ValueTrigger::unbounded(Action::Move),
                |_: &Grounded, &value| {
                    Some(match value {
                        value if value > 0.5 => Grounded::Right,
                        value if value < -0.5 => Grounded::Left,
                        _ => Grounded::Idle,
                    })
                },
            ),
    ));
}

#[derive(Actionlike, Clone, Reflect)]
enum Action {
    Move,
    Jump,
}

#[derive(Clone, Reflect)]
struct GroundedTrigger;

impl BoolTrigger for GroundedTrigger {
    type Param<'w, 's> = Query<'w, 's, &'static Transform>;

    fn trigger(&self, entity: Entity, transforms: &Self::Param<'_, '_>) -> bool {
        // Find the displacement between the target and this entity
        transforms.get(entity).unwrap().translation.y <= 0.
    }
}

#[derive(Clone, Copy, Component, Reflect)]
#[component(storage = "SparseSet")]
enum Grounded {
    Left = -1,
    Idle = 0,
    Right = 1,
}

#[derive(Clone, Component, Reflect)]
#[component(storage = "SparseSet")]
struct Falling {
    velocity: f32,
}

const PLAYER_SPEED: f32 = 200.;

fn walk(mut groundeds: Query<(&mut Transform, &Grounded)>, time: Res<Time>) {
    for (mut transform, grounded) in &mut groundeds {
        transform.translation.x += *grounded as i32 as f32 * time.delta_seconds() * PLAYER_SPEED;
    }
}

const GRAVITY: f32 = -1000.;

fn fall(mut fallings: Query<(&mut Transform, &mut Falling)>, time: Res<Time>) {
    for (mut transform, mut falling) in &mut fallings {
        let dt = time.delta_seconds();
        falling.velocity += dt * GRAVITY;
        transform.translation.y += dt * falling.velocity;
        println!("{}", falling.velocity);
    }
}
