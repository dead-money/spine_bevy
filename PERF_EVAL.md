# Performance evaluation and roadmap

Measured 2026-04-23 at commit `1665ff3` (Bevy 0.18) on an RTX 4090 and i9-14900K, Linux, X11/Vulkan, windowed. Re-measure before relying on these numbers for Bevy 0.19.

## Measurements

From `cargo run --release --example spine_stress -- --csv stress.csv` at perfect-square counts:

| N    | tick     | build    | total spine | frame budget left | FPS  | likely bottleneck above this point |
|------|----------|----------|-------------|--------------------|------|-------------------------------------|
| 1    | 0.07 ms  | 0.02 ms  | 0.09 ms     | 16.58 ms           | 657  | GPU & schedule overhead             |
| 50   | 0.19 ms  | 0.07 ms  | 0.25 ms     | 16.42 ms           | 570  | GPU (lots of headroom)              |
| 200  | 0.44 ms  | 0.19 ms  | 0.63 ms     | 16.04 ms           | 310  | GPU draw calls                      |
| 500  | 0.93 ms  | 0.46 ms  | 1.39 ms     | 15.28 ms           | 156  | render extract + draw calls         |
| 1000 | 1.72 ms  | 1.03 ms  | 2.75 ms     | 13.92 ms           | 84   | render extract + draw calls         |

The drop above about 500 instances is not Spine CPU work, which stays under 3 ms at 1000. The rest of the 16.67 ms frame goes to everything after `SpineSet::BuildMeshes`: extracting 1000 mesh assets, a bind-group switch per material (each skeleton owns one material per render command), and 1000+ draw calls.

## Before optimizing

- Profile a real game scene. The stress test runs identical spineboys; a game scene has 5 to 50 rigs with different animations next to sprites, particles, UI, and physics. Its bottleneck may be elsewhere.
- Pick a target, such as "60 fps with at most X ms for Spine", so there's a point to stop.
- Add a CI perf check. `spine_stress --csv` is a starting point: fail when p99 frame time at N=200 exceeds a threshold.

## Measurement work

1. **Frame-time percentiles in `spine_stress`.** It reports `Diagnostic::smoothed()`, which hides outliers. Record every frame time and print p50 / p95 / p99 / p99.9 on exit. A p99 above 16.67 ms drops frames even when the average is fine.
2. **Per-stage timing in the crate.** The timing systems (`mark_tick_start`, `mark_tick_end_and_build_start`, `mark_build_end`) live only in `spine_stress`. A `profile` feature would give every consumer the same numbers.
3. **Tracing.** A feature that names each `SpineSet` stage as a `puffin` or `tracy` scope, for flame graphs of upload, extract, prepare, and draw.
4. **GPU timings.** Bevy's `RenderDiagnosticsPlugin` exposes wgpu timestamp queries. Until `gpu_ms` is reported next to tick and build, "the bottleneck is the GPU" is an inference from the gap between `tick + build` and frame time.

## Likely wins

All of these target the build-to-draw path, where most of the frame went at N=1000.

5. **Shared materials.** Every render-command slot of every skeleton gets its own `SpineMaterial`, so 1000 spineboys are 1000 materials with identical texture, blend mode, and colors. Sharing one handle per distinct material could let Bevy batch the draws. This is probably the biggest GPU-side win without an architecture change. Check first whether Bevy batches `Mesh2d` draws with matching material handles; if not, this needs #9. A shared cache also needs care so despawning one skeleton doesn't free materials others use.
6. **Shared static meshes.** Skeletons in an identical, undeformed pose produce identical meshes and could share one `Handle<Mesh>`. Rare in practice, but cheap; pairs with #5.
7. **Skip off-screen and paused skeletons in `BuildMeshes`.** Cull against the viewport using per-skeleton bounds (`aggregate_bounds` in `examples/common/` could move into the crate).
8. **One mesh per skeleton.** Each render command is its own `Mesh2d`. Spineboy batches to one command, but rigs like dragon emit 14. Merging them into one mesh with index ranges would help those rigs. Defer until measurements show it matters.

## Larger changes

Only if the above isn't enough.

9. **Custom render command with manual batching.** Skip `Material2d` and batch all skeletons' geometry into a few draw calls keyed on texture and blend mode. This is how production Spine integrations for other engines render.
10. **GPU skinning.** Upload bone matrices and skin in the vertex shader, removing per-vertex CPU work. Requires runtime changes; only worth it for thousands of skeletons.
11. **GPU animation evaluation** for many instances of one rig with a small, known set of animations. Larger still than #10.

## Recommended order

1. Profile a real scene and set a target.
2. GPU timings (#4), the missing data point.
3. Shared materials (#5), the cheapest likely win.
4. Stop unless a real scene still misses the target.

## Already done

- `tick_spine_skeletons` runs skeletons in parallel with `par_iter_mut`: total Spine cost at N=1000 went from 10.4 ms to 2.7 ms.
- Initialized skeletons carry `SpineInitialized`, so the init system skips them.
- `write_mesh_from_command` rewrites mesh attributes in place instead of allocating new buffers each frame.
- The runtime batches adjacent commands with the same texture, blend mode, and color. Spineboy draws as one command.
