# Changelog

## 0.2 (2022-10-07)

### Added

- `Done::success()` and `Done::failure()` constructors
- `DoneTrigger::success()` and `DoneTrigger::failure()` constructors

### Changed

- `MachineState`s must implement `Bundle` and don't need to implement `Component`
- `DoneTrigger` and `Done` each have a `success` field. So, `DoneTrigger` will only trigger
when a `Done` component with a matching `success` field is added.
