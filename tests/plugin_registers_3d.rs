//! Headless smoke test for the 3D pipeline: a skeleton tagged with
//! [`SpineRender3d`] goes through init / tick / build-meshes without
//! panicking on the `None`-asset / `None`-state branches, just like the
//! 2D sibling test.

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

    // Spawn a 3D-tagged skeleton with a never-loaded asset handle. Init
    // observes `None`, leaves state = None; both build systems should
    // skip without panics.
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
    // A skeleton spawned without either marker should end up with
    // SpineRender2d inserted by the EnsureMarkers stage, so existing
    // 2D-only code keeps working after the 3D plugin lands.
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
