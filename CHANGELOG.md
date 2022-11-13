# Changelog

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
