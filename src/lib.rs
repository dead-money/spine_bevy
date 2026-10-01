//! Bevy 0.19 plugin for [`spine_runtime`]: load, animate and draw Spine 4.3
//! skeletons.
//!
//! Add [`SpinePlugin`], load a `.skel` or `.json` export as a
//! [`SpineSkeletonAsset`], and spawn a [`SpineSkeleton`] with it. The plugin
//! builds the runtime state once the asset loads, advances it every frame,
//! and draws it as child mesh entities.
//!
//! # Quick start (2D)
//!
//! ```no_run
//! use bevy::prelude::*;
//! use spine_bevy::{SpinePlugin, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings};
//!
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn(Camera2d);
//!     let skel: Handle<SpineSkeletonAsset> = asset_server
//!         .load_builder()
//!         .with_settings(|s: &mut SpineSkeletonLoaderSettings| {
//!             s.atlas_path = Some("spineboy/export/spineboy-pma.atlas".into());
//!         })
//!         .load("spineboy/export/spineboy-pro.skel");
//!     commands.spawn(SpineSkeleton::new(skel).with_initial_animation(0, "walk", true));
//! }
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(SpinePlugin)
//!     .add_systems(Startup, setup)
//!     .run();
//! ```
//!
//! # Quick start (3D)
//!
//! Spawn a [`SpineRender3d`] marker alongside the [`SpineSkeleton`] and use a
//! `Camera3d`. The rig is laid out in the entity's local XY plane (z = 0),
//! y-up and facing +Z, so it stands upright with an identity rotation. Spine
//! units are usually pixels; scale the `Transform` down to scene units. See
//! `examples/spineboy_walk_3d.rs`.
//!
//! # Controlling playback
//!
//! [`SpineSkeleton::play`], [`SpineSkeleton::set_skin`], and the
//! [`time_scale`](SpineSkeleton::time_scale), [`paused`](SpineSkeleton::paused)
//! and [`physics`](SpineSkeleton::physics) fields cover the common cases. Calls
//! made before the asset loads are queued. For queueing, mixing and track
//! entries, use [`SpineSkeleton::animation_state_mut`]; for bones and slots,
//! [`SpineSkeleton::skeleton_mut`]. Both return `None` until the asset loads.
//!
//! Animation events arrive as messages. [`SpineKeyframeEvent`] carries
//! keyframe events only; [`SpineStateEvent`] carries those plus track-entry
//! lifecycle events (start, interrupt, end, complete, dispose).
//!
//! ```no_run
//! use bevy::prelude::*;
//! use spine_bevy::{SpineKeyframeEvent, SpineSkeleton};
//!
//! fn log_events(mut events: MessageReader<SpineKeyframeEvent>, skeletons: Query<&SpineSkeleton>) {
//!     for ev in events.read() {
//!         let Some(skeleton) = skeletons.get(ev.entity).ok().and_then(|s| s.skeleton()) else {
//!             continue;
//!         };
//!         let name = &skeleton.data().events[ev.event.data.index()].name;
//!         info!("{name} at {}s", ev.event.time);
//!     }
//! }
//! ```
//!
//! # How it fits together
//!
//! [`SpinePlugin`] registers the asset loaders, the [`SpineMaterial`]
//! (`Material2d`) and [`SpineMaterial3d`] (`Material`) plugins, the two event
//! messages, and five chained system sets in `Update`:
//!
//! 1. [`SpineSet::EnsureMarkers`]: inserts [`SpineRender2d`] on skeletons
//!    that carry no render-mode marker.
//! 2. [`SpineSet::Init`]: builds runtime state for skeletons whose asset
//!    has loaded.
//! 3. [`SpineSet::Tick`]: advances and applies animations, updates world
//!    transforms, and renders each skeleton's `RenderCommand` list. Runs in
//!    parallel over skeletons.
//! 4. [`SpineSet::BuildMeshes`]: writes those commands into child mesh
//!    entities with a [`SpineMaterial`] or [`SpineMaterial3d`], depending on
//!    the render-mode marker.
//! 5. [`SpineSet::Events`]: forwards lifecycle and keyframe events as
//!    [`SpineStateEvent`] and [`SpineKeyframeEvent`] messages.
//!
//! Systems ordered `.before(SpineSet::Tick)` can change `time_scale` or
//! queue animations and see the result the same frame. Systems that read
//! bone positions should run `.after(SpineSet::Tick)`.
//!
//! # Atlas expectations
//!
//! The materials assume premultiplied-alpha textures. Spine exports usually
//! include a `*-pma.atlas` / `*-pma.png` pair beside the straight-alpha one.
//! The loaders derive `spineboy.atlas` from `spineboy-pro.skel`, which in
//! Spine's examples is the straight-alpha atlas; set
//! [`SpineSkeletonLoaderSettings::atlas_path`] to the PMA variant.

use bevy::asset::AssetApp;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

pub mod asset;
pub mod components;
pub mod material;
pub mod mesh;
pub mod systems;

pub use asset::{
    SpineAtlasAsset, SpineAtlasLoader, SpineAtlasLoaderError, SpineSkeletonAsset,
    SpineSkeletonJsonLoader, SpineSkeletonJsonLoaderError, SpineSkeletonJsonLoaderSettings,
    SpineSkeletonLoader, SpineSkeletonLoaderError, SpineSkeletonLoaderSettings,
};
pub use components::{
    PendingAnimation, SpineRender2d, SpineRender3d, SpineSkeleton, SpineSkeletonState,
};
pub use material::{SpineBlendMode, SpineColors, SpineMaterial, SpineMaterial3d, SpineMaterialKey};
pub use mesh::{build_spine_meshes, build_spine_meshes_3d};
pub use systems::{
    SpineInitialized, SpineKeyframeEvent, SpineSet, SpineStateEvent, drain_spine_events,
    ensure_spine_render_marker, initialize_spine_skeletons, tick_spine_skeletons,
};

/// Registers Spine assets, loaders, materials, messages and systems. Add
/// once, then spawn [`SpineSkeleton`] components.
#[derive(Default)]
pub struct SpinePlugin;

impl Plugin for SpinePlugin {
    fn build(&self, app: &mut App) {
        material::spine_material::register_spine_shader(app);
        material::spine_material_3d::register_spine_shader_3d(app);

        app.init_asset::<SpineAtlasAsset>()
            .init_asset::<SpineSkeletonAsset>()
            .init_asset_loader::<SpineAtlasLoader>()
            .init_asset_loader::<SpineSkeletonLoader>()
            .init_asset_loader::<SpineSkeletonJsonLoader>()
            .add_plugins(Material2dPlugin::<SpineMaterial>::default())
            .add_plugins(MaterialPlugin::<SpineMaterial3d>::default())
            .add_message::<SpineStateEvent>()
            .add_message::<SpineKeyframeEvent>()
            .configure_sets(
                Update,
                (
                    SpineSet::EnsureMarkers,
                    SpineSet::Init,
                    SpineSet::Tick,
                    SpineSet::BuildMeshes,
                    SpineSet::Events,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    ensure_spine_render_marker.in_set(SpineSet::EnsureMarkers),
                    initialize_spine_skeletons.in_set(SpineSet::Init),
                    tick_spine_skeletons.in_set(SpineSet::Tick),
                    build_spine_meshes.in_set(SpineSet::BuildMeshes),
                    build_spine_meshes_3d.in_set(SpineSet::BuildMeshes),
                    drain_spine_events.in_set(SpineSet::Events),
                ),
            );
    }
}
