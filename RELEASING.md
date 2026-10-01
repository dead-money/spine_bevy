# Releasing

How `spine_bevy` gets to crates.io.

## Rust version

`rust-version` in `Cargo.toml` tracks recent stable Rust, and never sits below
what the pinned Bevy needs. Bump it when the crate starts using something newer.

## CI

`ci.yml` runs fmt, clippy, test, and rustdoc (`-D warnings`) on every push to
`main` and every PR. It checks out `spine_runtime` beside this repo (a branch
with the same name if one exists, else `main`) and the pinned `spine-runtimes`
commit; bump that pin together with `spine_runtime`'s.

CI doesn't run `cargo package`: that builds against the `spine_runtime` on
crates.io rather than the sibling checkout, so it would fail any change that
needs unreleased runtime API. `cargo publish` in `release.yml` does that check
at release time.

## First release

crates.io can't set up Trusted Publishing for a crate that doesn't exist yet,
so 0.2.0 was published by hand with a `publish-new` API token, after
`spine_runtime` 0.2.0, then tagged `v0.2.0`. `release.yml` ran on the tag, saw
0.2.0 already published, and skipped.

Trusted Publishing is configured on crates.io → `spine_bevy` → Settings →
Trusted Publishing: repository `dead-money/spine_bevy`, workflow `release.yml`,
environment blank.

## Later releases

With `main` clean, CI green, and any newer `spine_runtime` this release needs
already published:

```sh
cargo release minor --dry-run
cargo release minor --execute
```

Per `release.toml`, it bumps the version, stamps `CHANGELOG.md`, commits, tags
`vX.Y.Z`, and pushes. The tag triggers `release.yml`, which runs the tests,
authenticates through Trusted Publishing, and publishes.

Each Bevy minor gets a new `spine_bevy` minor; keep the README's version table
current.
