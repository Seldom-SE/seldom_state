# Changelog

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
