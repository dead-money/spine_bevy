//! Bevy 0.19 integration for [`spine_runtime`].
//!
//! # Quick start (2D)
//!
//! ```no_run
//! use bevy::prelude::*;
//! use spine_bevy::{SpinePlugin, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings};
//!
//! fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     commands.spawn(Camera2d);
//!     let skel: Handle<SpineSkeletonAsset> = asset_server.load_with_settings(
//!         "spineboy/export/spineboy-pro.skel",
//!         |s: &mut SpineSkeletonLoaderSettings| {
//!             s.atlas_path = Some("spineboy/export/spineboy-pma.atlas".into());
//!         },
//!     );
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
//! `Camera3d`. The rig is laid out in the entity's local XY plane (z = 0);
//! rotate the entity's `Transform` to stand it upright. See
//! `examples/spineboy_walk_3d.rs`.
//!
//! # How it fits together
//!
//! [`SpinePlugin`] registers the asset loaders, the [`SpineMaterial`]
//! (`Material2d`) and [`SpineMaterial3d`] (`Material`) plugins, two
//! `Message` types for animation events, and five chained system sets in
//! `Update`:
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
//! queue animations and see the result the same frame.
//!
//! # Atlas expectations
//!
//! The materials assume premultiplied-alpha textures. Spine exports usually
//! include a `*-pma.atlas` / `*-pma.png` pair beside the straight-alpha one.
//! The loaders derive `spineboy.atlas` from `spineboy-pro.skel`, which in
//! Spine's examples is the straight-alpha atlas; set
//! [`SpineSkeletonLoaderSettings::atlas_path`] to the PMA variant.
//!
//! [`spine_runtime`]: https://github.com/dead-money/spine_runtime

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

/// Registers Spine assets, materials, messages and systems. Add once, then
/// spawn [`SpineSkeleton`] components.
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
