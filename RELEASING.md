# Releasing

## CI

`ci.yml` runs fmt, clippy, test, and rustdoc (`-D warnings`) for every push to
`main` and every PR. It checks out `spine_runtime` beside this repo (on a PR,
the `spine_runtime` branch with the same name if one exists; otherwise `main`)
and the pinned `spine-runtimes` commit. Bump that pin together with
`spine_runtime`'s.

CI doesn't run `cargo package`, which builds against the `spine_runtime` on
crates.io and would fail any change that needs unreleased runtime API.
`cargo publish` in `release.yml` makes that check at release time.

`rust-version` tracks recent stable Rust and never sits below what the pinned
Bevy needs.

## Cutting a release

Publish any `spine_runtime` version this release needs first, and point the
`spine_runtime` dependency's `version` in `Cargo.toml` at it. Then, with
[cargo-release](https://github.com/crate-ci/cargo-release) installed, `main`
clean, and CI green:

```sh
cargo release patch --dry-run   # or minor
cargo release patch --execute
```

It bumps the version, stamps `CHANGELOG.md`, commits, tags `vX.Y.Z`, and
pushes. The tag triggers `release.yml`, which tests and publishes through
crates.io Trusted Publishing. Afterward, check the crate page and the docs.rs
build.

Each Bevy minor gets a new `spine_bevy` minor; keep the README's version table
current. Keep `## [Unreleased]` in `CHANGELOG.md` current as PRs land.
