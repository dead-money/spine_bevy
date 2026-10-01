//! Headless smoke test: `SpinePlugin` builds and runs frames with a skeleton
//! whose asset never loads.

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

    // A never-loaded handle exercises every system's no-state branch.
    let handle: Handle<SpineSkeletonAsset> = Handle::default();
    app.world_mut().spawn(SpineSkeleton::new(handle));

    app.update();
    app.update();

    let mut query = app.world_mut().query::<&SpineSkeleton>();
    let count = query.iter(app.world()).count();
    assert_eq!(count, 1);
}
