//! Headless smoke tests for the 3D path and render-marker backfill.

use bevy::asset::AssetPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;

use spine_bevy::{SpinePlugin, SpineRender3d, SpineSkeleton, SpineSkeletonAsset};

#[test]
fn plugin_builds_and_ticks_an_empty_3d_skeleton_component() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(MeshPlugin)
        .add_plugins(SpinePlugin);

    // A never-loaded handle exercises every system's no-state branch.
    let handle: Handle<SpineSkeletonAsset> = Handle::default();
    app.world_mut()
        .spawn((SpineSkeleton::new(handle), SpineRender3d));

    app.update();
    app.update();

    let mut query = app.world_mut().query::<&SpineSkeleton>();
    let count = query.iter(app.world()).count();
    assert_eq!(count, 1);
}

#[test]
fn default_skeleton_gets_2d_marker_backfilled() {
    use spine_bevy::SpineRender2d;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(MeshPlugin)
        .add_plugins(SpinePlugin);

    let handle: Handle<SpineSkeletonAsset> = Handle::default();
    let entity = app.world_mut().spawn(SpineSkeleton::new(handle)).id();

    app.update();

    assert!(
        app.world().get::<SpineRender2d>(entity).is_some(),
        "SpineRender2d should have been backfilled"
    );
}
