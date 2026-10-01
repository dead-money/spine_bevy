# spine_bevy

[![CI](https://github.com/dead-money/spine_bevy/actions/workflows/ci.yml/badge.svg)](https://github.com/dead-money/spine_bevy/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/spine_bevy.svg)](https://crates.io/crates/spine_bevy)
[![docs.rs](https://docs.rs/spine_bevy/badge.svg)](https://docs.rs/spine_bevy)

A Bevy 0.19 plugin that loads, animates, and draws [Spine](https://esotericsoftware.com/) 4.3 skeletons. Each skeleton is an entity, drawn in 2D or 3D.

It builds on [`spine_runtime`](https://github.com/dead-money/spine_runtime), an unofficial Rust port of the Spine runtime. This crate turns that runtime's output into Bevy meshes and materials, and has no `unsafe` code.

Built for Dead Money's own games and mostly written by AI agents under human direction.

<p align="center">
  <img src="docs/celestial-circus-swing.gif" alt="celestial-circus rig playing the swing animation, captured live from the spine_browser example" width="640">
</p>
<p align="center"><em>The celestial-circus sample rig playing its swing animation in the <code>spine_browser</code> example.</em></p>

## You need a Spine Editor license

This crate's own code is MIT, but it depends on `spine_runtime`, a translation of Esoteric Software's `spine-cpp` under the [Spine Runtimes License Agreement](https://esotericsoftware.com/spine-runtimes-license). Under Section 2 of the [Spine Editor License Agreement](https://esotericsoftware.com/spine-editor-license):

- **Every developer who builds software with this crate needs their own [Spine Editor license](https://esotericsoftware.com/spine-purchase),** including to build and run the examples. Players of a game you ship don't need one.
- **Ship the license text.** Include the Spine Runtimes License Agreement in the documentation or other materials that come with your product.

If you're unsure whether your use is covered, ask Esoteric Software.

This release reads **Spine 4.3** exports, binary (`.skel`) or JSON, each paired with its `.atlas`. 4.2 exports won't load; re-export them from a 4.3 editor.

## Quick start

```toml
[dependencies]
bevy = "0.19"
spine_bevy = "0.2"
```

It needs Rust 1.99 or newer.

```rust
use bevy::prelude::*;
use spine_bevy::{SpinePlugin, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SpinePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Point the loader at the premultiplied-alpha atlas (see Atlases below).
    let skeleton: Handle<SpineSkeletonAsset> = asset_server
        .load_builder()
        .with_settings(|s: &mut SpineSkeletonLoaderSettings| {
            s.atlas_path = Some("spineboy/export/spineboy-pma.atlas".into());
        })
        .load("spineboy/export/spineboy-pro.skel");

    commands.spawn(SpineSkeleton::new(skeleton).with_initial_animation(0, "walk", true));
}
```

`examples/spineboy_walk.rs` is the runnable version. JSON exports load the same way, with `SpineSkeletonJsonLoaderSettings`.

### Playing animations and skins

`SpineSkeleton` is a component, so change it from any system:

```rust
fn jump_on_space(keys: Res<ButtonInput<KeyCode>>, mut skeletons: Query<&mut SpineSkeleton>) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }
    for mut skeleton in &mut skeletons {
        skeleton.play(0, "jump", false);
        skeleton.time_scale = 1.5;
    }
}
```

- `play(track, name, looping)` starts an animation, and `set_skin(name)` switches skins. Both work before the asset finishes loading; they apply once it does.
- `with_initial_animation` and `with_initial_skin` set them at spawn.
- `available_animations()` and `available_skins()` list what the rig has.
- `time_scale`, `paused`, and `physics` control playback.
- For anything else, such as queuing animations or blending tracks, `animation_state_mut()` and `skeleton_mut()` give you the `spine_runtime` objects directly.

To change something on the frame it should take effect, order your system `.before(SpineSet::Tick)`.

### Events

Animation events keyed in the editor (footsteps, hit frames) arrive as Bevy messages, tagged with the skeleton's entity:

```rust
use spine_bevy::SpineKeyframeEvent;

fn footsteps(mut events: MessageReader<SpineKeyframeEvent>, skeletons: Query<&SpineSkeleton>) {
    for e in events.read() {
        let Some(skeleton) = skeletons.get(e.entity).ok().and_then(|s| s.skeleton()) else {
            continue;
        };
        if skeleton.data().events[e.event.data.index()].name == "footstep" {
            // play a sound
        }
    }
}
```

`SpineStateEvent` carries playback events: an animation starting, being interrupted, ending, or completing a loop.

### 3D

Add `SpineRender3d` beside the skeleton and use a `Camera3d`. The rig draws as a flat, unlit plane you can place and rotate with its `Transform`:

```rust
commands.spawn((SpineSkeleton::new(skeleton), SpineRender3d, Transform::from_xyz(0.0, 0.0, -5.0)));
```

2D and 3D skeletons can share one app. `examples/spineboy_walk_3d.rs` shows a full scene.

## Atlases

The materials expect **premultiplied-alpha** textures. Most Spine exports include a `*-pma.atlas` / `*-pma.png` pair beside the regular one; point the loader at it, as in the Quick start. Without settings, the loader looks for an atlas named after the skeleton (dropping `-pro`, `-ess`, or `-ios`). For a straight-alpha atlas, premultiply the PNGs in your asset pipeline.

## Performance

Skeletons update in parallel, and a rig usually draws in a few draw calls; spineboy is typically one. On an i9-14900K, 50 skeletons cost about 0.25 ms of CPU per frame and 1000 about 2.75 ms. Past roughly 1000 on screen, the GPU side becomes the limit. `cargo run --release --example spine_stress` measures it on your machine.

## Version compatibility

| Bevy | Spine | spine_bevy |
|------|-------|------------|
| 0.19 | 4.3   | 0.2        |
| 0.18 | 4.3   | `f2a00bb`  |
| 0.18 | 4.2   | `8c7be8a`  |

Each Bevy minor needs its own `spine_bevy` release. The 0.18 rows are the last commits on that Bevy version; pair the 4.2 one with `spine_runtime`'s `v0.1.0` tag.

## Examples

The examples use the rigs from Esoteric's [`spine-runtimes`](https://github.com/EsotericSoftware/spine-runtimes) repository, cloned beside this one. That art is licensed separately and doesn't ship with this crate. A clone of this repo also builds against a `spine_runtime` checkout beside it.

```sh
git clone https://github.com/dead-money/spine_runtime ../spine_runtime
git clone -b 4.3 https://github.com/EsotericSoftware/spine-runtimes ../spine-runtimes
cargo run --example spine_browser
```

- `spine_browser` is a gallery of every example rig. **Space** / **Shift+Space** switch rigs, **N** switches animations, **S** switches skins, **R** restarts, **+** / **-** change speed, **Esc** quits. `--assets <path>` or `SPINE_EXAMPLES_DIR` points it at the rigs.
- `spineboy_walk` is the Quick start.
- `spineboy_walk_3d` draws spineboy in a 3D scene with a ground plane, a light, and an orbiting camera.
- `spineboy_screenshot` saves frame 60 to a PNG and exits (`SPINE_SCREENSHOT`, `SPINE_SCREENSHOT_FRAMES`).
- `spine_stress` fills the screen with spineboys. **]** / **[** change the count, and `--csv path.csv` logs timings per frame. Run it with `--release`.

To record a GIF like the one above, `spine_browser` can save a frame sequence:

```sh
SPINE_BROWSER_RECORD_DIR=/tmp/frames SPINE_BROWSER_RECORD_FRAMES=120 SPINE_BROWSER_RECORD_WARMUP=45 \
cargo run --example spine_browser -- --rig celestial-circus-pro --anim swing --width 640 --height 360
ls /tmp/frames/frame_*.png | awk 'NR%2==1' | xargs convert -delay 3 -loop 0 -layers OptimizePlus clip.gif
```

## Licensing

Source in this repository is © 2026 Dead Money under the [MIT License](./LICENSE). It's original work; no `spine-cpp` code is translated here.

`spine_runtime`, which this crate depends on, is a `spine-cpp` derivative under the [Spine Runtimes License Agreement](https://esotericsoftware.com/spine-runtimes-license), not MIT. Its terms, including the Spine Editor license requirement above, apply to anything built with this crate.

## Acknowledgements

Built on [Bevy](https://bevy.org/) and [`spine_runtime`](https://github.com/dead-money/spine_runtime), a port of [Esoteric Software](https://esotericsoftware.com/)'s Spine runtime. The upstream [spine-runtimes](https://github.com/EsotericSoftware/spine-runtimes) repository is the source of truth for runtime behavior. Report runtime bugs to `spine_runtime`; report Bevy integration bugs here.
