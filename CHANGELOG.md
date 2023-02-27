# Changelog

## 0.4 (2022-02-26)

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
