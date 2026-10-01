# Releasing

How `spine_bevy` gets to crates.io. **It isn't cleared to publish yet.**

## The gate

`publish = false` in `Cargo.toml` blocks every path to crates.io: `cargo
publish` refuses, and so does the tag-triggered `release.yml`. Two things have
to happen first:

1. Esoteric Software agrees to Spine runtime derivatives being published there
   (tracked in `spine_runtime`'s `RELEASING.md`).
2. `spine_runtime` is published at the version this crate requires
   (`version = "0.2"` on its dependency). Until then, `cargo package` can't
   resolve it, which is why CI doesn't run that check here.

## Rust version

`rust-version` in `Cargo.toml` tracks recent stable Rust, and never sits below
what the pinned Bevy needs. Bump it when the crate starts using something newer.

## CI

`ci.yml` runs fmt, clippy, test, and rustdoc (`-D warnings`) on every push to
`main` and every PR. It checks out `spine_runtime` beside this repo (a branch
with the same name if one exists, else `main`) and the pinned `spine-runtimes`
commit; bump that pin together with `spine_runtime`'s.

## First release (manual)

crates.io can't set up Trusted Publishing for a crate that doesn't exist yet,
so the first version is published by hand, after `spine_runtime` 0.2.

1. On a branch, remove `publish = false` from `Cargo.toml` and stamp
   `CHANGELOG.md`: under `## [Unreleased]`, add `## [0.2.0] - <date>` above the
   notes. Merge it.
2. From a clean `main` with CI green:

   ```sh
   cargo publish --dry-run
   CARGO_REGISTRY_TOKEN=<token> cargo publish
   git tag v0.2.0 && git push origin v0.2.0
   ```

   Create the token at crates.io → Account → API Tokens with the
   `publish-new` scope, and revoke it afterward. `release.yml` runs on the tag,
   sees 0.2.0 is already published, and skips.
3. On crates.io → `spine_bevy` → Settings → Trusted Publishing, add a GitHub
   publisher: repository `dead-money/spine_bevy`, workflow `release.yml`,
   environment blank.
4. Switch the README's Quick start from git dependencies to versions, and add
   crates.io and docs.rs badges.

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
