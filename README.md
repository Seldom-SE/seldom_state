# `seldom_state`

[![Crates.io](https://img.shields.io/crates/v/seldom_state.svg)](https://crates.io/crates/seldom_state)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/Seldom-SE/seldom_state#license)
[![Crates.io](https://img.shields.io/crates/d/seldom_state.svg)](https://crates.io/crates/seldom_state)

`seldom_state` is a component-based state machine plugin for Bevy. It's useful for AI, player state,
and other entities that occupy various states. It allows for greater reusability of state logic
between entities, compared to managing mutually-exclusive components directly in your systems.

A *state* is a component attached to an entity that defines its current behavior, such as `Jumping`
or `Stunned`. A *trigger* is a system that checks information about entities in the world, such as
`near_position` or `health_below_threshold`. A *transition* links two states: one to transition
from, and one to transition to; once a given trigger has occurred. A *state machine* is a component
attached to an entity that keeps track of that entity's transitions, and automatically changes the
entity's state according to those transitions.

State machines are created like so:

```Rust
commands.spawn((
    // ... (other inserts)
    MyInitialState::new(),
    StateMachine::default()
        .trans::<MyInitialState, _>(my_trigger_1, my_state_2)
        .trans::<AnyState, _>(my_trigger_3, my_state_4)
        .trans_builder(my_trigger_5, |my_state_6: &MyState6, trigger_data| {
            make_state_7(my_state_6, trigger_data)
        })
        .on_enter::<MyState7>(move |entity| entity.insert(my_bundle.clone()))
        .on_exit::<MyState7>(|entity| entity.remove::<MyBundle>())
        // etc.
));
```

For more complete examples, see the `examples` directory. The `chase.rs` example is written like a
guide, so it is good for learning. If you need help, feel free to ping me on
[the Bevy Discord server](https://discord.com/invite/bevy) (`@Seldom`)! If anything needs
improvement, feel free to submit an issue or pr!

## Features

- State machine component with user-defined states and triggers
- 30 built-in triggers
    - `always`: always triggers
    - `NotTrigger`, `AndTrigger`, and `OrTrigger`: combines triggers with boolean logic
    - `done`: triggers when the `Done` component is added to the entity
    - 24 more triggers enabled by the `leafwing_input` feature: `action_data`, `axis_pair`,
    `axis_pair_length_bounds`, `axis_pair_max_length`, `axis_pair_min_length`,
    `axis_pair_rotation_bounds`, `axis_pair_unbounded`, `clamped_axis_pair`,
    `clamped_axis_pair_length_bounds`, `clamped_axis_pair_max_length`,
    `clamped_axis_pair_min_length`, `clamped_axis_pair_rotation_bounds`,
    `clamped_axis_pair_unbounded`, `clamped_value`, `clamped_value_max`, `clamped_value_min`,
    `clamped_value_unbounded`, `just_pressed`, `just_released`, `pressed`, `value`, `value_max`,
    `value_min`, and `value_unbounded`
    - `on_event`: triggers when it reads an event of the given type
    - Bevy's [built-in run conditions](https://docs.rs/bevy/latest/bevy/ecs/schedule/common_conditions/index.html)
    also work as triggers.
- `AnyState` state, that can be used in type parameters to represent any state
- Transition builders that allow dataflow from outgoing states and triggers to incoming states
(`StateMachine::trans_builder`)
- Automatically perform behavior upon entering or exiting states (`StateMachine::on_enter`,
`StateMachine::on_exit`, `StateMachine::command_on_enter` and `StateMachine::command_on_exit`)

## Comparison with [`big-brain`](https://github.com/zkat/big-brain)

Finite state machine is an old and well-worn pattern in game AI, so its strengths and limitations
are known. It is good for entities that:

1. Do not have a huge number of interconnected states, since the number of transitions can grow
quadratically. Then it becomes easy to forget to add a transition, causing difficult bugs.
2. Act rigidly, like the enemies in Spelunky, who act according to clear triggers such as
got-jumped-on-by-player and waited-for-5-seconds and are predictable to the player, and unlike the
dwarves in Dwarf Fortress, who weigh their options of what to do before taking an action and feel
lively.

`seldom_state` is a finite state machine implementation, so it may not be suitable for all types of
game AI. If you need a solution that works with more complex states and transitions, then you may
want to implement
[a behavior tree](https://www.gamedeveloper.com/programming/behavior-trees-for-ai-how-they-work) (I
had little luck turning existing implementations into a Bevy plugin without forking them). If you
need a solution that operates on fuzzy logic, and do not need to define which transitions should be
allowed, then I recommend `big-brain`. If you need fuzzy logic *and* discrete transitions, you may
want to implement a fuzzy state machine. If you need discrete transitions, but not fuzzy logic,
consider `seldom_state`!

`seldom_state` is not just an AI crate, though. So, you may want to use `big-brain` for your
enemies' AI, and `seldom_state` to manage state for your player, and control enemies' animation, or
something.

## Design patterns

`seldom_state` is rather versatile, so some problems may be solved in multiple ways. If you're lost,
here is some advice.

### I have an entity whose animations depend on behavior

There are a few solutions to this. The most straightforward is to add the animations to the entity
with `on_enter`. This works for animation systems that rigidly follow behavior, such as the player
controller in a 2D fighter or a basic enemy controller. Of course, this is rigid, and anything that
the animations must remember between states must be handled manually. In a platformer like Celeste
that has multiple animations for a single state (lets assume it has states `Grounded`, `Airborne`,
`Dashing`, and `Climbing`), you might manage some animations, like the dashing animation, through
`on_enter`, and others, like the walk cycle, through systems. Or, you might manage all animations
through systems. This is up to preference.

On the other hand, if your animations are less constrained to behavior, consider using multiple
state machines, as in the next section:

### My entity needs to be in multiple states at once

Consider a 2D platformer, where the player has a sword. The player can run and jump around, and they
can swing the sword. So whether you're running, jumping, or dashing, you always swing the sword the
same way, independently of movement state. In this case, you might want to have a movement state
machine and an attack state machine. Since entities can only have one state machine, spawn another
entity (as a child, I would suggest) with its own state machine, and capture the original `Entity`
in closures in `command_on_enter` and `command_on_exit`.

However, perhaps your states are not so independent. Maybe attacking while dashing puts the player
in a `PowerAttack` state, or the attack cooldown doesn't count down while moving. Depending on the
scale of the dependency, you might want to just have your state machines communicate through
commands and observing each other's states, or you might want to combine the state machines,
permuting relevant states into states like `DashAttack` and `IdleAttackCooldown`.

Also, consider managing one set of states through a state machine and another through systems.

### I have some other problem that's difficult to express through the `StateMachine` API

Remember that `StateMachine` is component-based, so you can solve some problems normally through
Bevy's ECS. Instead of `on_enter::<MyState>` you can use `Added<MyState>` in a system, and you can
even change state manually through `remove` and `insert` commands. If you do change state manually,
callbacks like `on_enter` will not be called, and you will have to make sure that the state machine
remains in exactly one state at a time. Else, it will panic.

## Usage

Add to your `Cargo.toml`

```toml
# Replace * with your desired version

[dependencies]
seldom_state = "*"
```

See the `chase.rs` example for further usage.

## Compatibility

| Bevy | `leafwing-input-manager` | `seldom_state` |
| ---- | ------------------------ | -------------- |
| 0.12 | 0.11                     | 0.8            |
| 0.11 | 0.10                     | 0.7            |
| 0.10 | 0.9                      | 0.5 - 0.6      |
| 0.9  | 0.8                      | 0.4            |
| 0.9  |                          | 0.3            |
| 0.8  |                          | 0.1 - 0.2      |

## License

`seldom_state` is dual-licensed under MIT and Apache 2.0 at your option.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
