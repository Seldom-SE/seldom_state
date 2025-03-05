# Changelog

## 0.13 (2024-03-05)

### Added

- `OneOfState` is a state that matches any state given in the tuple in its type parameter
- `NotState` is a state that matches any state other than the one given in its type parameter
- `Trigger::ignore_and` is like `Trigger::and`, but throws away the output of the first trigger
- `StateMachinePlugin::schedule` changes the schedule in which transitions occur

### Changed

- The behavior of `StateMachine::trans_builder` has changed. Now, builders can no longer reject a
transition (instead of returning `Option<MyState>`, they return `MyState`). Instead, builders are
now mutable systems that accept `Trans` (context for the transition) as an input.

### Fixed

- Fixed a panic when `StateMachine` is removed during a transition

## 0.12 (2024-12-18)

### Changed

- Updated `bevy` to 0.15
- Updated `leafwing-input-manager` to 0.16 (see their `RELEASES.md` for a migration guide)

## 0.11 (2024-07-09)

### Changed

- Updated `bevy` to 0.14
- Updated `leafwing-input-manager` to 0.14
- Renamed `Trigger` to `EntityTrigger` to avoid conflict with `bevy::prelude::Trigger`

### Removed

- `seldom_fn_plugin` integration

## 0.10 (2024-02-21)

### Added

- `Out` definitions to trigger combinator functions' return types

### Changed

- Updated `bevy` to 0.13
- Updated `leafwing-input-manager` to 0.13

### Fixed

- `on_event` trigger could detect events that occured before the latest transition

## 0.9 (2024-01-12)

### Added

- `IntoSystem` trait, which allows systems which return `bool`, `Option`, or `Result`, and accept
`Entity` or nothing as input, to be used as triggers
- `TriggerIn` trait, which allows systems which accept nothing as input to be used as triggers
- `TriggerOut` trait, which allows triggers to output `bool` or `Option` without `BoolTrigger` and
`OptionTrigger`
- `SystemTrigger` type, which is also used for system triggers

### Changed

- `trans` and `trans_builder` each have a new type parameter, which should simply be elided with `_`
- Replaced all built-in trigger types with system equivalents
- Renamed `MachineState` to `EntityState`
- `Trigger` has an `Out` type and `init` and `check` functions, instead of `Param`, `Ok`, and `Err`
types and a `trigger` function

### Removed

- `BoolTrigger` and `OptionTrigger`

## 0.8 (2023-11-12)

### Added

- `Clone` and `Copy` impls for triggers and other types
- `PartialEq` and `Eq` impl for `Never`

### Changed

- Updated `bevy` to 0.12
- Updated `leafwing-input-manager` to 0.11

### Fixed

- The `leafwing_input` feature was a default feature

## 0.7 (2023-07-15)

### Changed

- Updated `bevy` to 0.11
- Updated `leafwing-input-manager` to 0.10

## 0.6.3 (2023-06-16)

### Fixed

- `PressedTrigger` firing only once per press
- `AxisPairTrigger` and `ClampedAxisPairTrigger` failing to fire without rotation bounds

## 0.6.2 (2023-06-09)

### Changed

- `trigger` module is public

### Fixed

- Transitions sometimes failing when there are multiple entities with state machines

## 0.6.1 (2023-05-15)

### Added

- `EventTrigger` re-export, making it accessible

## 0.6 (2023-05-07)

### Added

- `Component` bound to `MachineState`
- `StateMachine::on_enter` and `StateMachine::on_exit` that accept `Fn(&mut EntityCommands)`
- `StateMachine::command_on_enter` and `StateMachine::command_on_exit` that accept a `Command`
- `AndTrigger` and `OrTrigger` that combine triggers with boolean logic
- `Trigger::not`, `Trigger::and`, and `Trigger::or` combinators that construct `NotTrigger`,
`AndTrigger`, and `OrTrigger` respectively
- `EventTrigger<E>` that triggers upon reading the event
- `StateMachine::set_trans_logging` for logging transitions
- `StateMachine::with_state` to register states that are not used in any transitions

### Changed

- `StateMachine::new` has been replaced with `StateMachine::default`, and you must insert the
initial state manually
- `StateMachine::trans_builder`'s callback accepts a reference to the current state
- `StateMachine` no longer tracks the current state, so you may add and remove states manually
- Transitions have priority in the order they are added to the `StateMachine`
- `Trigger::Param` is a `ReadOnlySystemParam` instead of `SystemParam`, and `Trigger::trigger`
accepts it as an owned value, allowing them to use `EventReader`, `Local`, etc

### Removed

- `TriggerPlugin`, `trigger_plugin`, `TriggerInputPlugin` and `trigger_input_plugin`; `Triggers` no
longer need to be registered
- `StateMachine::insert_on_enter` and `StateMachine::remove_on_exit`
- `Reflect` bound from `MachineState` and `Trigger`

## 0.5 (2023-03-12)

### Changed

- Updated `bevy` to 0.10
- Updated `leafwing-input-manager` to 0.9
- `AnyState` is uninhabited, so it cannot be constructed

### Fixed

- `insert_on_enter` inserting on exit instead of enter

## 0.4 (2023-02-26)

### Added

- `StateMachine::trans_builder` builds a state based on data provided by the trigger
- `AnyState` state that can be used in type parameters to represent any state
- `leafwing_input` feature for input-related triggers: `ValueTrigger`, `ClampedValueTrigger`,
`AxisPairTrigger`, `ClampedAxisPairTrigger`, `JustPressedTrigger`, `PressedTrigger`,
`JustReleasedTrigger`, `ReleasedTrigger`, and `ActionDataTrigger`
- `InputTriggerPlugin<Actionlike>` for easily registering input-related triggers
- `OptionTrigger` and `BoolTrigger` offer simpler APIs than `Trigger`
- `Never` type for `Trigger::Err` when the trigger always occurs

### Changed

- A panic will occur when using a trigger that hasn't been registered
- `Trigger` has associated types `Ok` and `Err`, and `Trigger::trigger` returns
a `Result<Self::Ok, Self::Err>`, which are used in transition builders

### Removed

- `Clone` and `FromReflect` requirements for `Trigger`
- `Clone` from `StateMachine`
- `FromReflect` re-export from `prelude`

## 0.3 (2022-11-12)

### Changed

- Updated `bevy` to 0.9
- `Trigger::Param<'w, 's>` has lifetime parameters
- `Trigger::trigger` accepts `&Self::Param::Fetch::Item`
instead of `&StaticSystemParam<Self::Param>`
- `Done` and `DoneTrigger` are enums with `Success` and `Failure` variants

### Removed

- `StaticSystemParam` re-export from `prelude`
- `Done::success()` and `Done::failure()` constructors
- `DoneTrigger::success()` and `DoneTrigger::failure()` constructors

## 0.2.2 (2022-11-07)

### Changed

- Creating multiple transitions from the same `MachineState` and `Trigger` type doesn't panic

## 0.2.1 (2022-10-30)

### Added

- `StateMachine::insert_on_enter` configures the `StateMachine` to insert a bundle
when entering a state
- `StateMachine::remove_on_exit` configures the `StateMachine` to remove a bundle
when exiting a state
- `StaticSystemParam` and `FromReflect` are re-exported in `prelude`

## 0.2 (2022-10-07)

### Added

- `Done::success()` and `Done::failure()` constructors
- `DoneTrigger::success()` and `DoneTrigger::failure()` constructors

### Changed

- `MachineState`s must implement `Bundle` and don't need to implement `Component`
- `DoneTrigger` and `Done` each have a `success` field. So, `DoneTrigger` will only trigger
when a `Done` component with a matching `success` field is added.
