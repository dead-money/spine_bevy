//! Headless smoke test: SpinePlugin registers, systems wire into Update,
//! and one frame of a pending-asset skeleton doesn't panic on the `None`
//! state branch.

use bevy::asset::AssetPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;

use spine_bevy::{SpinePlugin, SpineSkeleton, SpineSkeletonAsset};

#[test]
fn plugin_builds_and_ticks_an_empty_skeleton_component() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(MeshPlugin)
        .add_plugins(SpinePlugin);

    // Spawn a component whose asset handle is default (never-loaded). Init
    // should observe `None` from Assets::get and leave state = None; tick
    // and mesh-build should skip; no panics.
    let handle: Handle<SpineSkeletonAsset> = Handle::default();
    app.world_mut().spawn(SpineSkeleton::new(handle));

    app.update();
    app.update();

    let mut query = app.world_mut().query::<&SpineSkeleton>();
    let count = query.iter(app.world()).count();
    assert_eq!(count, 1);
}
