//! `spineboy_walk` drawn through the 3D pipeline ([`SpineRender3d`]), standing
//! on a ground plane under a perspective camera that orbits the rig.
//!
//! ```bash
//! cargo run --example spineboy_walk_3d
//! ```
//!
//! Set `SPINE_SCREENSHOT_3D` to a PNG path to capture one frame after
//! `SPINE_SCREENSHOT_3D_FRAMES` (default 90) frames and exit.

use std::f32::consts::TAU;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use spine_bevy::{
    SpinePlugin, SpineRender3d, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings,
};

mod common;

/// Maps spineboy (about 400 units tall) to about 2 scene units.
const SKELETON_SCALE: f32 = 0.005;
const CAMERA_RADIUS: f32 = 6.0;
const CAMERA_HEIGHT: f32 = 3.0;
const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 1.2, 0.0);

#[derive(Component)]
struct OrbitCamera;

fn main() {
    let asset_root = common::resolve_asset_root(None)
        .expect("spineboy_walk_3d: clone https://github.com/EsotericSoftware/spine-runtimes alongside this repo");

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root.to_string_lossy().into_owned(),
                ..Default::default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    .add_plugins(SpinePlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, orbit_camera);

    if let Some(cfg) =
        common::ScreenshotConfig::from_env("SPINE_SCREENSHOT_3D", "SPINE_SCREENSHOT_3D_FRAMES", 90)
    {
        common::install_screenshot_driver(&mut app, cfg);
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let start = Vec3::new(CAMERA_RADIUS * 0.5, CAMERA_HEIGHT, CAMERA_RADIUS * 0.87);
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(start).looking_at(CAMERA_TARGET, Vec3::Y),
        OrbitCamera,
    ));

    // Lights only the ground; `SpineMaterial3d` is unlit.
    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.27, 0.3),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // The materials expect premultiplied alpha; the stock `spineboy.atlas` is straight.
    let skel_handle: Handle<SpineSkeletonAsset> = asset_server
        .load_builder()
        .with_settings(|settings: &mut SpineSkeletonLoaderSettings| {
            settings.atlas_path = Some("spineboy/export/spineboy-pma.atlas".to_string());
        })
        .load("spineboy/export/spineboy-pro.skel");

    // Spine is y-up in the XY plane, so the rig stands upright without rotation.
    commands.spawn((
        SpineSkeleton::new(skel_handle).with_initial_animation(0, "walk", true),
        SpineRender3d,
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(SKELETON_SCALE)),
    ));
}

fn orbit_camera(time: Res<Time>, mut q: Query<&mut Transform, With<OrbitCamera>>) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    // About 18 s per revolution.
    let angle = (time.elapsed_secs() * 0.35 + TAU / 6.0) % TAU;
    tf.translation = Vec3::new(
        angle.sin() * CAMERA_RADIUS,
        CAMERA_HEIGHT,
        angle.cos() * CAMERA_RADIUS,
    );
    tf.look_at(CAMERA_TARGET, Vec3::Y);
}
