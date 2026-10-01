//! First visual example: loads spineboy-pro, plays the `walk` animation on
//! track 0. Points the asset root at the upstream `spine-runtimes/examples`
//! directory so we don't duplicate rig binaries into this crate.
//!
//! Run from `spine_bevy/`:
//!
//! ```bash
//! cargo run --example spineboy_walk
//! ```

use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use spine_bevy::{SpinePlugin, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings};

mod common;

fn main() {
    let asset_root = common::resolve_asset_root(None)
        .expect("spineboy_walk: clone https://github.com/EsotericSoftware/spine-runtimes alongside this repo");

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_root.to_string_lossy().into_owned(),
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(SpinePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        // Spineboy is tall and positioned with origin at foot. Shift the
        // camera up so the full rig sits in frame.
        Transform::from_xyz(0.0, 200.0, 0.0),
    ));

    // Override the atlas path to the PMA variant — the shader assumes
    // premultiplied-alpha textures, which the stock `spineboy.atlas` is
    // not. `-pma.png` ships pre-multiplied.
    let skel_handle: Handle<SpineSkeletonAsset> = asset_server
        .load_builder()
        .with_settings(|settings: &mut SpineSkeletonLoaderSettings| {
            settings.atlas_path = Some("spineboy/export/spineboy-pma.atlas".to_string());
        })
        .load("spineboy/export/spineboy-pro.skel");

    commands.spawn((
        SpineSkeleton::new(skel_handle).with_initial_animation(0, "walk", true),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}
