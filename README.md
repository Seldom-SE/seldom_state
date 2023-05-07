# `seldom_state`

[![Crates.io](https://img.shields.io/crates/v/seldom_state.svg)](https://crates.io/crates/seldom_state)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/Seldom-SE/seldom_state#license)
[![Crates.io](https://img.shields.io/crates/d/seldom_state.svg)](https://crates.io/crates/seldom_state)

`seldom_state` is a component-based state machine plugin for Bevy. It's useful for AI,
player state, and other entities that occupy various states. It allows for greater reusability
of state logic between entities, compared to managing mutually-exclusive components directly
in your systems.

A *state* is a bundle attached to an entity that defines its current behavior, such as `Jumping`
or `Stunned`. A *trigger* is a type that checks information about entities in the world,
such as `NearPosition` or `HealthBelowThreshold`. A *transition* links two states:
one to transition from, and one to transition to; once a given trigger has occurred.
A *state machine* is a component attached to an entity that keeps track of that entity's
transitions, and automatically changes the entity's state according to those transitions.

State machines are created like so:

```Rust
commands.spawn((
    // ... (other inserts)
    MyInitialState::new()
    StateMachine::default()
        .trans::<MyInitialState>(my_trigger_1, my_state_2)
        .trans::<AnyState>(my_trigger_3, my_state_4)
        .trans_builder(my_trigger_5, |my_state_6: &MyState6, trigger_data| {
            make_state_7(my_state_6, trigger_data)
        })
        .on_enter::<MyState7>(move |entity| entity.insert(my_bundle.clone()))
        .on_exit::<MyState7>(|entity| entity.remove::<MyBundle>())
        // etc.
));
```

For more complete examples, see the `examples` directory. The `chase.rs` example is written
like a guide, so it is good for learning. If you need help, feel free to ping me
on [the Bevy Discord server](https://discord.com/invite/bevy) (`@Seldom`)! If any of the docs
need improvement, feel free to submit an issue or pr!

## Features

* State machine component with user-defined states and triggers
* 12 built-in triggers
    * `AlwaysTrigger`: always triggers
    * `NotTrigger`: contains a trigger, which it negates
    * `DoneTrigger`: triggers when the user adds the `Done` component to the entity
    * 9 more triggers enabled by the `leafwing_input` feature: `ValueTrigger`,
    `ClampedValueTrigger`, `AxisPairTrigger`, `ClampedAxisPairTrigger`, `JustPressedTrigger`,
    `PressedTrigger`, `JustReleasedTrigger`, `ReleasedTrigger`, and `ActionDataTrigger`
* `AnyState` state, that can be used in type parameters to represent any state
* Transition builders that allow dataflow from triggers to states (`StateMachine::trans_builder`)
* Automatically insert bundles upon entering a state, and remove others upon exiting
(`StateMachine::insert_on_enter` and `StateMachine::remove_on_exit`)

## Future Work

- [X] Warn or panic when using a trigger whose plugin hasn't been added
- [X] [`leafwing_input_manager`](https://github.com/Leafwing-Studios/leafwing-input-manager)
integration
- [X] Dataflow from triggers to states using state builders
- [X] Transitions that can transition from any state
- [X] Automatically insert bundle on transition
- [ ] Built-in timer trigger (I might implement this, and wouldn't mind it)
- [X] More flexible, composable triggers, such as `And<A: Trigger, B: Trigger>(A, B)` (I might
implement this, and definitely want it)
- [X] `EventTrigger` that listens for events, possibly with additional logic (I might implement
this, and definitely want it)
- [X] Replace `insert_on_enter` and `remove_on_exit` with `on_enter` and `on_exit` which each take
an `impl Fn(&mut EntityCommands)`

## Comparison with [`big-brain`](https://github.com/zkat/big-brain)

Finite state machine is an old and well-worn pattern in game AI, so its strengths and limitations
are known. It is good for entities that:

1. Do not have a large number of interconnected states, since the number of transitions can grow
quadratically. Then it becomes easy to forget to add a transition, causing difficult bugs.
2. Act rigidly, like the enemies in Spelunky, who act according to clear triggers such as
got-jumped-on-by-player and waited-for-5-seconds, and unlike the dwarves in Dwarf Fortress,
who weigh their options of what to do before taking an action.

`seldom_state` is a finite state machine implementation, so it may not be suitable for all types
of game AI. If you need a solution that works with more complex states and transitions,
then you may want to implement
[a behavior tree](https://www.gamedeveloper.com/programming/behavior-trees-for-ai-how-they-work)
(I had little luck turning existing implementations into a Bevy plugin without forking them).
If you need a solution that operates on fuzzy logic, and do not need to define
which transitions should be allowed, then I recommend `big-brain`. If you need fuzzy logic
*and* defined transitions, you may want to implement a fuzzy state machine. Otherwise, this crate
will likely work for you!

`seldom_state` is not just an AI crate, though. So, you may want to use `big-brain`
for your enemies' AI, and `seldom_state` to manage state for your player, and control enemies'
animation, or something.

## Usage

Add to your `Cargo.toml`

```toml
# Replace * with your desired version

[dependencies]
seldom_state = "*"
```

See the `chase.rs` example for further usage.

## Compatibility

| Bevy | `seldom_state` |
| ---- | -------------- |
| 0.10 | 0.5            |
| 0.9  | 0.3 - 0.4      |
| 0.8  | 0.1 - 0.2      |

## License

`seldom_state` is dual-licensed under MIT and Apache 2.0 at your option.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.
