# spine_bevy

Bevy 0.19 plugin for Spine 4.3 skeletons, built on the sibling runtime crate
`spine_runtime` (`~/deadmoney/spine_runtime/`, `path = "../spine_runtime"`). A secondary
consumer of the runtime; hommlet is the primary one.

## Invariants

- **Two-crate split.** Spine behavior (loading, posing, animation, render commands) lives
  in `spine_runtime`, which stays engine-agnostic. This crate is Bevy glue: assets,
  components, systems, meshes, materials. `spine_runtime` is ours and freely editable;
  fix runtime bugs there, not here. This crate is `unsafe_code = forbid`.
- **The frame order follows spine-cpp:** `AnimationState::update`, `apply`,
  `Skeleton::update`, `update_world_transform`, then `render`. Physics needs
  `Skeleton::update`.
- **One Bevy minor per release.** A Bevy upgrade bumps this crate's minor and adds a row to
  the README's version table.
- **PMA only.** The materials assume premultiplied-alpha atlases.
- `~/deadmoney/spine-runtimes/` is the upstream reference and the examples' rigs.
  **Read-only.**

## Cross-repo changes

CI checks out `spine_runtime` on the branch with this PR's name if one exists, else `main`.
For a change that spans both repos, push same-named branches to both and merge the runtime
PR first.

## Attribution

Do not add `Co-Authored-By: Claude` trailers to commits or "Generated with Claude Code"
footers to PR bodies. Author lines and PR bodies stay clean.

## Process

Same as `spine_runtime`'s `CLAUDE.md`: merge commits (`gh pr merge --merge
--delete-branch`), terse commit messages and PR bodies, sparse comments that say what the
code can't. Keep `CHANGELOG.md`'s `[Unreleased]` current. Publishing is gated by
`publish = false`; see `RELEASING.md`. `rust-version` tracks recent stable.

## Pointers

- Try rigs live: `cargo run --example spine_browser`.
- Throughput: `cargo run --release --example spine_stress`; [`PERF_EVAL.md`](./PERF_EVAL.md)
  has the measurements and the improvement roadmap.
