// In this game, you can move with the left and right arrow keys, and jump with space.
// `leafwing-input-manager` handles the input.

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use seldom_state::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputManagerPlugin::<Action>::default(),
            StateMachinePlugin::default(),
        ))
        .add_systems(Startup, init)
        .add_systems(Update, (walk, fall))
        .run();
}

const JUMP_VELOCITY: f32 = 500.;

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn((
        // From `leafwing-input-manager`
        InputMap::default()
            .with_axis(Action::Move, VirtualAxis::horizontal_arrow_keys())
            .with_axis(
                Action::Move,
                GamepadControlAxis::new(GamepadAxis::LeftStickX),
            )
            .with(Action::Jump, KeyCode::Space)
            .with(Action::Jump, GamepadButton::South),
        // This state machine achieves a very rigid movement system. Consider a state machine for
        // whatever parts of your player controller that involve discrete states. Like the movement
        // in Castlevania and Celeste, and the attacks in a fighting game.
        Grounded::Idle,
        StateMachine::default()
            // Whenever the player presses jump, jump
            .trans::<Grounded, _>(
                just_pressed(Action::Jump),
                Falling {
                    velocity: JUMP_VELOCITY,
                },
            )
            // When the player hits the ground, idle
            .trans::<Falling, _>(grounded, Grounded::Idle)
            // When the player is grounded, set their movement direction
            .trans_builder(
                value_unbounded(Action::Move),
                |trans: Trans<Grounded, _>| {
                    let value = trans.out;

                    match value {
                        value if value > 0.5 => Grounded::Right,
                        value if value < -0.5 => Grounded::Left,
                        _ => Grounded::Idle,
                    }
                },
            ),
        Sprite::from_image(asset_server.load("player.png")),
        Transform::from_xyz(500., 0., 0.),
    ));
}

#[derive(Actionlike, Clone, Eq, Hash, PartialEq, Reflect, Debug)]
enum Action {
    #[actionlike(Axis)]
    Move,
    Jump,
}

fn grounded(In(entity): In<Entity>, fallings: Query<(&Transform, &Falling)>) -> bool {
    let (transform, falling) = fallings.get(entity).unwrap();
    transform.translation.y <= 0. && falling.velocity <= 0.
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
        transform.translation.x += *grounded as i32 as f32 * time.delta_secs() * PLAYER_SPEED;
    }
}

const GRAVITY: f32 = -1000.;

fn fall(mut fallings: Query<(&mut Transform, &mut Falling)>, time: Res<Time>) {
    for (mut transform, mut falling) in &mut fallings {
        let dt = time.delta_secs();
        falling.velocity += dt * GRAVITY;
        transform.translation.y += dt * falling.velocity;
    }
}
