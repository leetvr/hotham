# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## UNRELEASED
### Changed
- Fixed default hand glTF files so offsets are not required when applied to grip pose - @rasmusgo [#271](https://github.com/leetvr/hotham/pull/271)

## [0.2] - 2022-05-10
### Added
- Developers can now add their own application name, version and OpenXR layers to the `Engine` - @jmgao [#197](https://github.com/leetvr/hotham/pull/197)

### Changed
- `Panel`s are now independent from `egui`, thanks to @jmgao's fantastic work. `Panel` can now have variable sizes and developers are able to add their own custom content. Existing functionality has been moved to `UiPanel`. -  @jmgao [#188](https://github.com/leetvr/hotham/issues/188).

### Fixed
- Hotham and Hotham Simulator now no longer have hard dependencies on Vulkan Validation layers. [#182](https://github.com/leetvr/hotham/issues/182)
- Hotham Simulator no longer segfaults on close - @jmgao [#185](https://github.com/leetvr/hotham/issues/185)
- Android debug build is no longer broken - @jmgao [#186](https://github.com/leetvr/hotham/issues/186)
- Fixed ANR on volume up / down - @jmgao [#190](https://github.com/leetvr/hotham/issues/190)
- Bump to `ndk-glue` - @jmgao [#201](https://github.com/leetvr/hotham/issues/201)

### Maintenance
- Significant CI improvements, including `clippy` - @davidkuhta [#160](https://github.com/leetvr/hotham/issues/160)


## 0.1 - 2022-03-01
Initial release!
