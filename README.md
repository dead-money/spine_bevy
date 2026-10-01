# spine_bevy

[![CI](https://github.com/dead-money/spine_bevy/actions/workflows/ci.yml/badge.svg)](https://github.com/dead-money/spine_bevy/actions/workflows/ci.yml)

A Bevy 0.19 plugin that loads, animates, and draws [Spine](https://esotericsoftware.com/) 4.3 skeletons. Skeletons are entities; they render through Bevy's 2D sprite pipeline or its 3D pipeline, chosen per entity.

It builds on [`spine_runtime`](https://github.com/dead-money/spine_runtime), the renderer-agnostic Rust port of the Spine runtime. This crate maps that runtime's `RenderCommand` stream onto Bevy meshes and materials, and sets `unsafe_code = "forbid"`.

Built for Dead Money's own games and mostly written by AI agents under human direction.

<p align="center">
  <img src="docs/celestial-circus-swing.gif" alt="celestial-circus rig playing the swing animation, captured live from the spine_browser example" width="640">
</p>
<p align="center"><em>The celestial-circus sample rig playing its swing animation, captured live from the <code>spine_browser</code> example.</em></p>

## You need a Spine Editor license

This crate inherits the license obligations of `spine_runtime`, a derivative of Esoteric Software's `spine-cpp`. Distribution is governed by the [Spine Editor License Agreement](https://esotericsoftware.com/spine-editor-license) and the [Spine Runtimes License Agreement](https://esotericsoftware.com/spine-runtimes-license), the same obligation every official Spine runtime carries:

- **Every end user of software built with this crate needs their own [Spine Editor license](https://esotericsoftware.com/spine-purchase).**
- **Keep the notices.** Every source file carries Esoteric Software's copyright block, and `LICENSE` reproduces the Spine Runtimes License verbatim.

If your use case is in doubt, check the [Spine licensing page](https://esotericsoftware.com/spine-purchase) or ask Esoteric Software.

`main` targets **Spine 4.3** exports, binary `.skel` or JSON `.json`, each paired with a `.atlas`. The runtime rejects 4.2 exports, so re-export from a 4.3 editor.

## Quick start

```toml
[dependencies]
bevy = "0.19"
spine_runtime = { git = "https://github.com/dead-money/spine_runtime" }
spine_bevy = { git = "https://github.com/dead-money/spine_bevy" }
```

Neither crate is on crates.io yet.

```rust
use bevy::prelude::*;
use spine_bevy::{
    SpinePlugin, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpinePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Without settings, the loader derives the atlas path from the skeleton
    // stem (stripping -pro / -ess / -ios). Here we point it at the PMA atlas.
    // `.json` exports work the same way with SpineSkeletonJsonLoaderSettings.
    let skel: Handle<SpineSkeletonAsset> = asset_server
        .load_builder()
        .with_settings(|s: &mut SpineSkeletonLoaderSettings| {
            s.atlas_path = Some("spineboy/export/spineboy-pma.atlas".into());
        })
        .load("spineboy/export/spineboy-pro.skel");

    commands.spawn(SpineSkeleton::new(skel).with_initial_animation(0, "walk", true));
}
```

`examples/spineboy_walk.rs` is the runnable version of this snippet.

## What it does

- **`SpinePlugin`** registers the asset loaders, the 2D (`Material2d`) and 3D (`Material`) PMA materials, the per-frame system sets, and two message streams for animation lifecycle and keyframe events.
- **Assets.** `SpineAtlasAsset` and `SpineSkeletonAsset` load through Bevy's `AssetServer`, and PNG pages resolve as dependencies. The atlas asset doubles as the `TextureId(u32) → Handle<Image>` table the renderer uses.
- **`SpineSkeleton`** holds per-instance state: skeleton, animation state, and renderer. Spawn it with a `Handle<SpineSkeletonAsset>`; the runtime state builds once the asset finishes loading. Helpers cover the common cases:
  - `play(track, name, looping)` starts an animation.
  - `set_skin(name)` swaps skins and re-resolves slot attachments.
  - `with_initial_animation` and `with_initial_skin` set spawn-time defaults.
  - `available_animations()` and `available_skins()` list what the rig has.
  - `time_scale`, `physics`, and `paused` control playback.
  - `animation_state_mut()` and `skeleton_mut()` reach the runtime directly when the helpers aren't enough, for example to set a `TrackEntry`'s `additive` or `mix_interpolation`.
- **2D or 3D per entity.** `SpineRender2d` (the default, inserted for you) draws through the sprite pipeline. Add `SpineRender3d` instead to draw through the 3D pipeline. One `App` can host both side by side.
- **PMA materials.** `SpineMaterial` (2D) and `SpineMaterial3d` (3D) share one shader: premultiplied-alpha blending, a pipeline specialization per blend mode (Normal, Additive, Multiply, Screen), and tint-black through the runtime's dark color. Both are unlit. The 3D variant disables back-face culling and depth writes, and has no shadow or prepass.
- **Events.** `SpineStateEvent` (`Start`, `Interrupt`, `End`, `Complete`, `Dispose`) and `SpineKeyframeEvent` (timeline keyframes) arrive as Bevy `Message`s tagged with the source entity.

Out of scope for now:

- **Straight-alpha atlases.** The materials assume PMA; see [Atlas expectations](#atlas-expectations).
- **Lit materials.** Spine's light and dark colors already bake the authored tint, and the atlas is premultiplied, so PBR lighting on top would double-count both. A lit variant could sit beside the shipped one if a real need shows up.
- **Billboarding.** In 3D a skeleton is a flat plane in its local XY. `Transform` rotates the whole rig, but there's no built-in camera-facing mode; a system that rotates the parent each frame does it.
- **An inspector or editor UI.**

## How it fits Bevy

Each frame the plugin runs five ordered system sets in `Update`:

1. **`SpineSet::EnsureMarkers`** inserts `SpineRender2d` on any `SpineSkeleton` without a render-mode marker, so a plain `commands.spawn(SpineSkeleton::new(...))` draws in 2D.
2. **`SpineSet::Init`** builds the runtime state for skeletons whose assets just loaded, then applies any spawn-time animation and skin.
3. **`SpineSet::Tick`** advances time, applies timelines, updates world transforms, and refills the renderer's command buffer.
4. **`SpineSet::BuildMeshes`** turns the `&[RenderCommand]` into child entities, mutating meshes in place via `Assets::get_mut`. `build_spine_meshes` handles `SpineRender2d` entities with `Mesh2d` / `SpineMaterial`; `build_spine_meshes_3d` handles `SpineRender3d` entities with `Mesh3d` / `SpineMaterial3d`. Children are reused across frames, and ones beyond this frame's command count are hidden rather than despawned.
5. **`SpineSet::Events`** drains lifecycle and keyframe events into `SpineStateEvent` and `SpineKeyframeEvent`.

Order your own systems `.before(SpineSet::Tick)` to change `time_scale` or queue animations on the frame they should take effect.

- **One `RenderCommand`, one child entity.** Each child gets a small per-index offset along the skeleton's local +Z. In 2D, `Transparent2d`'s back-to-front sort uses it directly. In 3D, `Transparent3d` sorts by camera distance with depth writes off, so slot order holds unless the camera views the skeleton's plane at a steep grazing angle.
- **Shared data.** `SkeletonData` is behind an `Arc`. Ten instances of one rig are ten `Arc::clone`s, not ten loads.
- **Few draw calls.** The runtime's batcher merges adjacent runs that share texture, blend mode, and color, and that holds through to Bevy. Spineboy usually draws as one mesh.
- **Parallel tick.** The tick stage runs over skeletons with `par_iter_mut`. On an i9-14900K, per-skeleton CPU cost settles around 3 µs: 0.08 ms for one skeleton, 0.25 ms for 50, 2.75 ms for 1000. Past about 1000 instances, mesh upload and draw calls on the GPU side dominate.

## Atlas expectations

The materials assume **premultiplied-alpha** textures and use the PMA blend equations (`ONE, ONE_MINUS_SRC_ALPHA` for Normal, `ONE, ONE` for Additive, and so on). Most Spine exports ship `*-pma.atlas` / `*-pma.png` beside the straight-alpha pair, so prefer those.

For a straight-alpha atlas, either point the loader at a PMA variant (as in the quick start), or premultiply the PNGs in your asset pipeline. Reading the atlas's `pma:` flag and premultiplying on load is a planned follow-up.

## Version compatibility

| Bevy | Spine | spine_bevy  |
|------|-------|-------------|
| 0.19 | 4.3   | `main`      |
| 0.18 | 4.3   | `f2a00bb`   |
| 0.18 | 4.2   | `8c7be8a`   |

The crate pins a Bevy minor because the render path depends on `Material2d` / `Material` specialization and the `Mesh2d` / `Mesh3d` extraction model, which change between releases. The 0.18 rows are the last commits on that Bevy version; pair the 4.2 one with `spine_runtime`'s `v0.1.0` tag.

## Building

The examples load the canonical rigs from a sibling clone of [`spine-runtimes`](https://github.com/EsotericSoftware/spine-runtimes). That art is licensed separately and doesn't ship with this crate. CI pins the same upstream `4.3` commit as `spine_runtime`.

```sh
git clone -b 4.3 https://github.com/EsotericSoftware/spine-runtimes ../spine-runtimes
cargo run --example spine_browser
```

`spine_browser` and `spine_stress` also take the asset root from `--assets <path>` or `SPINE_EXAMPLES_DIR`.

- `cargo run --example spine_browser` is an interactive gallery of every rig under the examples directory. The camera fits each rig's bounds as you switch.
  - **Space** / **Shift+Space**: next / previous rig
  - **N** / **Shift+N**: next / previous animation
  - **S** / **Shift+S**: next / previous skin
  - **R**: restart the animation
  - **+** / **-**: playback speed
  - **Esc**: quit
- `cargo run --example spineboy_walk` is the minimal load, play, and render example.
- `cargo run --example spineboy_walk_3d` draws the same rig through `SpineMaterial3d` on a perspective `Camera3d`, with a ground plane, a directional light, and an orbiting camera. Opting in is one component: spawn `SpineRender3d` beside `SpineSkeleton`. `SPINE_SCREENSHOT_3D` and `SPINE_SCREENSHOT_3D_FRAMES` take a screenshot and exit.
- `cargo run --example spineboy_screenshot` runs N frames, writes a PNG through Bevy's `Screenshot` API, and exits. `SPINE_SCREENSHOT` and `SPINE_SCREENSHOT_FRAMES` set the path and frame.
- `cargo run --release --example spine_stress` spawns spineboy in a square grid with per-instance time offsets. **]** and **[** step the count up and down (1, 4, 9, 16, …), **R** resets, **Esc** quits. The HUD shows fps, tick ms, and build ms, and `--csv path.csv` writes `frame,count,fps,tick_ms,build_ms` per frame. Use `--release`; debug builds skew the numbers.

### Recording animated previews

`spine_browser` can also record a frame sequence for previews like the GIF at the top. Pin a rig and animation, set the window size, capture N frames, then stitch them. The GIF above came from:

```sh
mkdir -p /tmp/celestial_frames
SPINE_BROWSER_RECORD_DIR=/tmp/celestial_frames \
SPINE_BROWSER_RECORD_FRAMES=120 \
SPINE_BROWSER_RECORD_WARMUP=45 \
cargo run --example spine_browser -- \
    --rig celestial-circus-pro --anim swing --width 640 --height 360

# Every other frame, ~30 fps, about 1 MB.
ls /tmp/celestial_frames/frame_*.png | awk 'NR%2==1' | xargs \
    convert -delay 3 -loop 0 -layers OptimizePlus docs/your-clip.gif
```

`SPINE_BROWSER_RECORD_WARMUP` gives the camera time to settle on the rig before recording starts.

## Testing

```sh
cargo test
cargo clippy --all-targets
cargo fmt --check
```

`tests/plugin_registers.rs` and `tests/plugin_registers_3d.rs` run the plugin in a minimal headless app for each pipeline. They check registration, asset loading, render-marker backfill, and one update tick.

## Licensing

Distributed under the [Spine Runtimes License Agreement](https://esotericsoftware.com/spine-runtimes-license). See [`LICENSE`](./LICENSE) for the full text.

Copyright © 2013-2025 Esoteric Software LLC. Bevy integration © Dead Money LLC, published under the same license.

## Acknowledgements

Built on [Bevy](https://bevy.org/) and [`spine_runtime`](https://github.com/dead-money/spine_runtime), a port of [Esoteric Software](https://esotericsoftware.com/)'s Spine runtime. The upstream [spine-runtimes](https://github.com/EsotericSoftware/spine-runtimes) repository is the source of truth for runtime behavior. Report runtime bugs to `spine_runtime`; report integration bugs here.
