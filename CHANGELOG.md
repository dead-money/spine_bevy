# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). While the crate is
pre-1.0, any `0.x` release may contain breaking changes.

## [Unreleased]

### Changed

- Corrected and tightened the API docs and comments throughout.

## [0.2.0] - 2026-10-01

### Changed

- **Breaking:** targets Bevy 0.19 and Spine 4.3, through `spine_runtime` 0.2.
  4.2 exports no longer load.
- The crate is renamed from `dm_spine_bevy` to `spine_bevy`.
- Licensed under MIT. This crate's code is original rather than a `spine-cpp`
  derivative; `spine_runtime`, which it depends on, stays under the Spine
  Runtimes License, so its Spine Editor license requirement still applies.

## [0.1.0]

Bevy 0.18 and Spine 4.2, as `dm_spine_bevy`. Never published; the last commit
is `8c7be8a`.
