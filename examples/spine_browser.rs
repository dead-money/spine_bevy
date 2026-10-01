//! Gallery of the rigs in upstream `spine-runtimes/examples/`. Cycle rigs,
//! animations, and skins from the keyboard; the camera follows the bounds of
//! the current pose.
//!
//! ## Controls
//!
//! - Space / Shift+Space: next / previous rig
//! - N / Shift+N: next / previous animation
//! - S / Shift+S: next / previous skin
//! - R: restart the current animation
//! - + / -: speed up / slow down playback (0.1x to 4x)
//! - Esc: quit
//!
//! ## Asset root
//!
//! The sample art lives in [`EsotericSoftware/spine-runtimes`] and is
//! licensed separately. The browser looks for its `examples/` directory in
//! this order:
//!
//! 1. `--assets <path>`
//! 2. `SPINE_EXAMPLES_DIR`
//! 3. `../spine-runtimes/examples`
//! 4. `./spine-runtimes/examples`
//!
//! If none exists, it prints a clone command and exits with an error.
//!
//! Other flags: `--rig <substring>`, `--anim <name>`, `--skin <name>`, and
//! `--width <px> --height <px>` for a fixed window size.
//!
//! ## Capture
//!
//! - `SPINE_BROWSER_SCREENSHOT=<png>`: save one frame and exit. The frame is
//!   `SPINE_BROWSER_SCREENSHOT_FRAMES` (default 120).
//! - `SPINE_BROWSER_RECORD_DIR=<dir>`: write `frame_NNNN.png` for
//!   `SPINE_BROWSER_RECORD_FRAMES` frames (default 90) after
//!   `SPINE_BROWSER_RECORD_WARMUP` frames (default 30), then exit.
//!
//! [`EsotericSoftware/spine-runtimes`]: https://github.com/EsotericSoftware/spine-runtimes

use std::path::PathBuf;
use std::process::ExitCode;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::WindowResolution;

use spine_bevy::{
    SpinePlugin, SpineSet, SpineSkeleton, SpineSkeletonAsset, SpineSkeletonLoaderSettings,
};

mod common;
use common::RigEntry;

#[derive(Resource)]
struct Browser {
    rigs: Vec<RigEntry>,
    current_rig: usize,
    /// Empty until the current rig's asset loads.
    animations: Vec<String>,
    current_animation: usize,
    /// Empty until the current rig's asset loads.
    skins: Vec<String>,
    current_skin: usize,
    time_scale: f32,
    /// Respawned on every rig change.
    skeleton_entity: Option<Entity>,
    /// Eased toward `target_view` each frame.
    current_view: View,
    /// Bounds of the current pose plus a margin.
    target_view: View,
    /// Set on rig change; `refresh_metadata` clears it once the asset loads.
    metadata_dirty: bool,
    /// `--anim` and `--skin`, consumed by the first metadata refresh so
    /// later rigs start on their first animation and skin.
    initial_anim: Option<String>,
    initial_skin: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct View {
    center: Vec2,
    /// Visible world units, vertically. Width follows the window aspect.
    height: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            center: Vec2::new(0.0, 200.0),
            height: 800.0,
        }
    }
}

#[derive(Component)]
struct HudText;

const VIEW_MARGIN: f32 = 1.15;
/// Camera easing per 60 Hz frame; settles in about 10 frames.
const VIEW_LERP: f32 = 0.15;
const MIN_VIEW_HEIGHT: f32 = 100.0;

/// `--key value` or `--key=value`; unknown keys are ignored.
#[derive(Default, Debug, Clone)]
struct Cli {
    assets: Option<PathBuf>,
    /// Substring of the starting rig's label, e.g. `celestial`.
    rig: Option<String>,
    /// Unknown names fall back to the first animation.
    anim: Option<String>,
    skin: Option<String>,
    /// Physical pixels. Both must be set to take effect.
    window_width: Option<u32>,
    window_height: Option<u32>,
}

fn parse_cli() -> Cli {
    let mut cli = Cli::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let (key, value) = if let Some((k, v)) = a.split_once('=') {
            (k.to_string(), Some(v.to_string()))
        } else {
            (a, args.next())
        };
        match key.as_str() {
            "--assets" => cli.assets = value.map(PathBuf::from),
            "--rig" => cli.rig = value,
            "--anim" => cli.anim = value,
            "--skin" => cli.skin = value,
            "--width" => cli.window_width = value.and_then(|v| v.parse().ok()),
            "--height" => cli.window_height = value.and_then(|v| v.parse().ok()),
            _ => {}
        }
    }
    cli
}

fn main() -> ExitCode {
    let cli = parse_cli();

    let asset_root = match common::resolve_asset_root(cli.assets.clone()) {
        Ok(p) => p,
        Err(message) => {
            eprintln!("spine_browser: {message}");
            return ExitCode::from(1);
        }
    };

    let rigs = common::discover_rigs(&asset_root);
    if rigs.is_empty() {
        eprintln!(
            "spine_browser: no rigs found under {}.\n\
             Expected structure: <root>/<rig>/export/<rig>.skel + <rig>.atlas",
            asset_root.display()
        );
        return ExitCode::from(1);
    }
    eprintln!(
        "spine_browser: discovered {} rig(s) under {}",
        rigs.len(),
        asset_root.display()
    );

    let initial_rig = cli
        .rig
        .as_ref()
        .and_then(|needle| rigs.iter().position(|r| r.label.contains(needle.as_str())))
        .unwrap_or(0);
    if let Some(needle) = &cli.rig
        && initial_rig == 0
        && !rigs[0].label.contains(needle.as_str())
    {
        eprintln!(
            "spine_browser: --rig {needle:?} matched no rig; starting at {}",
            rigs[0].label
        );
    }

    let mut window_plugin = WindowPlugin::default();
    if let (Some(w), Some(h)) = (cli.window_width, cli.window_height) {
        window_plugin.primary_window = Some(Window {
            resolution: WindowResolution::new(w, h),
            title: "spine_browser".into(),
            resizable: false,
            ..Default::default()
        });
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root.to_string_lossy().into_owned(),
                ..Default::default()
            })
            .set(ImagePlugin::default_nearest())
            .set(window_plugin),
    )
    .add_plugins(SpinePlugin)
    .insert_resource(Browser {
        rigs,
        current_rig: initial_rig,
        animations: Vec::new(),
        current_animation: 0,
        skins: Vec::new(),
        current_skin: 0,
        time_scale: 1.0,
        skeleton_entity: None,
        current_view: View::default(),
        target_view: View::default(),
        metadata_dirty: true,
        initial_anim: cli.anim,
        initial_skin: cli.skin,
    })
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            handle_input,
            refresh_metadata.after(SpineSet::Init),
            live_fit_camera.after(SpineSet::Tick),
            update_hud,
        ),
    );

    if let Some(cfg) = common::ScreenshotConfig::from_env(
        "SPINE_BROWSER_SCREENSHOT",
        "SPINE_BROWSER_SCREENSHOT_FRAMES",
        120,
    ) {
        common::install_screenshot_driver(&mut app, cfg);
    }

    // Frame sequence for assembling GIFs.
    if let Ok(dir) = std::env::var("SPINE_BROWSER_RECORD_DIR") {
        let dir = PathBuf::from(dir);
        let frames: u32 = std::env::var("SPINE_BROWSER_RECORD_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(90);
        let warmup: u32 = std::env::var("SPINE_BROWSER_RECORD_WARMUP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        std::fs::create_dir_all(&dir).expect("create record dir");
        app.insert_resource(RecordConfig {
            out_dir: dir,
            target_frames: frames,
            warmup_frames: warmup,
            current_frame: 0,
            captured: 0,
            done_frame: None,
        });
        app.add_systems(Update, record_driver);
    }

    app.run();

    ExitCode::SUCCESS
}

/// Writes `frame_NNNN.png` into `out_dir` for `target_frames` frames, after
/// `warmup_frames` frames for the camera to settle, then exits 60 frames
/// later so the async PNG writes finish.
#[derive(Resource)]
struct RecordConfig {
    out_dir: PathBuf,
    target_frames: u32,
    warmup_frames: u32,
    current_frame: u32,
    captured: u32,
    done_frame: Option<u32>,
}

fn record_driver(
    mut commands: Commands,
    mut cfg: ResMut<RecordConfig>,
    mut exit: MessageWriter<AppExit>,
) {
    cfg.current_frame += 1;
    const EXIT_GRACE_FRAMES: u32 = 60;

    if let Some(done) = cfg.done_frame {
        if cfg.current_frame >= done + EXIT_GRACE_FRAMES {
            exit.write(AppExit::Success);
        }
        return;
    }

    if cfg.current_frame <= cfg.warmup_frames {
        return;
    }

    let path = cfg.out_dir.join(format!("frame_{:04}.png", cfg.captured));
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    cfg.captured += 1;

    if cfg.captured >= cfg.target_frames {
        info!(
            "spine_browser: captured {} frames into {}",
            cfg.captured,
            cfg.out_dir.display()
        );
        cfg.done_frame = Some(cfg.current_frame);
    }
}

fn setup(mut commands: Commands, mut browser: ResMut<Browser>, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Transform::from_translation(browser.current_view.center.extend(0.0)),
    ));
    commands.spawn((
        HudText,
        Text::new("loading…"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            left: px(12),
            ..default()
        },
    ));
    spawn_current_rig(&mut commands, &mut browser, &asset_server);
}

fn spawn_current_rig(commands: &mut Commands, browser: &mut Browser, asset_server: &AssetServer) {
    if let Some(prev) = browser.skeleton_entity.take() {
        commands.entity(prev).despawn();
    }
    let rig = browser.rigs[browser.current_rig].clone();
    let atlas = rig.atlas_relpath.clone();
    let handle: Handle<SpineSkeletonAsset> = asset_server
        .load_builder()
        .with_settings(move |s: &mut SpineSkeletonLoaderSettings| {
            s.atlas_path = Some(atlas.clone());
        })
        .load(rig.skel_relpath.clone());
    let mut sk = SpineSkeleton::new(handle);
    sk.time_scale = browser.time_scale;
    let entity = commands.spawn(sk).id();
    browser.skeleton_entity = Some(entity);
    browser.metadata_dirty = true;
    browser.animations.clear();
    browser.skins.clear();
}

fn handle_input(
    mut commands: Commands,
    mut browser: ResMut<Browser>,
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    asset_server: Res<AssetServer>,
    mut sk_query: Query<&mut SpineSkeleton>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if keys.just_pressed(KeyCode::Space) {
        let n = browser.rigs.len();
        browser.current_rig = if shift {
            (browser.current_rig + n - 1) % n
        } else {
            (browser.current_rig + 1) % n
        };
        browser.current_animation = 0;
        browser.current_skin = 0;
        spawn_current_rig(&mut commands, &mut browser, &asset_server);
        return;
    }

    if keys.just_pressed(KeyCode::KeyN) && !browser.animations.is_empty() {
        let n = browser.animations.len();
        browser.current_animation = if shift {
            (browser.current_animation + n - 1) % n
        } else {
            (browser.current_animation + 1) % n
        };
        if let Some(entity) = browser.skeleton_entity
            && let Ok(mut sk) = sk_query.get_mut(entity)
        {
            let anim = browser.animations[browser.current_animation].clone();
            sk.play(0, anim, true);
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyS) && !browser.skins.is_empty() {
        let n = browser.skins.len();
        browser.current_skin = if shift {
            (browser.current_skin + n - 1) % n
        } else {
            (browser.current_skin + 1) % n
        };
        if let Some(entity) = browser.skeleton_entity
            && let Ok(mut sk) = sk_query.get_mut(entity)
        {
            let skin = browser.skins[browser.current_skin].clone();
            if let Err(err) = sk.set_skin(skin.clone()) {
                warn!("spine_browser: set_skin({skin:?}) failed: {err:?}");
            } else if !browser.animations.is_empty() {
                // `set_skin` resets slots to the setup pose; replaying
                // restores the animated attachments.
                let anim = browser.animations[browser.current_animation].clone();
                sk.play(0, anim, true);
            }
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyR)
        && let Some(entity) = browser.skeleton_entity
        && let Ok(mut sk) = sk_query.get_mut(entity)
        && !browser.animations.is_empty()
    {
        let anim = browser.animations[browser.current_animation].clone();
        sk.play(0, anim, true);
        return;
    }

    // `=` is unshifted `+` on most layouts.
    let inc = keys.just_pressed(KeyCode::NumpadAdd) || keys.just_pressed(KeyCode::Equal);
    let dec = keys.just_pressed(KeyCode::NumpadSubtract) || keys.just_pressed(KeyCode::Minus);
    if inc {
        browser.time_scale = (browser.time_scale * 1.25).clamp(0.1, 4.0);
        push_time_scale(&browser, &mut sk_query);
    } else if dec {
        browser.time_scale = (browser.time_scale / 1.25).clamp(0.1, 4.0);
        push_time_scale(&browser, &mut sk_query);
    }
}

fn push_time_scale(browser: &Browser, sk_query: &mut Query<&mut SpineSkeleton>) {
    if let Some(entity) = browser.skeleton_entity
        && let Ok(mut sk) = sk_query.get_mut(entity)
    {
        sk.time_scale = browser.time_scale;
    }
}

fn refresh_metadata(mut browser: ResMut<Browser>, mut sk_query: Query<&mut SpineSkeleton>) {
    if !browser.metadata_dirty {
        return;
    }
    let Some(entity) = browser.skeleton_entity else {
        return;
    };
    let Ok(mut sk) = sk_query.get_mut(entity) else {
        return;
    };
    let (Some(anims), Some(skins)) = (sk.available_animations(), sk.available_skins()) else {
        return;
    };

    let anims: Vec<String> = anims.iter().map(|s| (*s).to_string()).collect();
    let skins: Vec<String> = skins.iter().map(|s| (*s).to_string()).collect();

    if let Some(want) = browser.initial_anim.take()
        && let Some(idx) = anims.iter().position(|n| n == &want)
    {
        browser.current_animation = idx;
    }
    if let Some(want) = browser.initial_skin.take()
        && let Some(idx) = skins.iter().position(|n| n == &want)
    {
        browser.current_skin = idx;
    }

    browser.current_animation = browser.current_animation.min(anims.len().saturating_sub(1));
    browser.current_skin = browser.current_skin.min(skins.len().saturating_sub(1));
    browser.metadata_dirty = false;

    if browser.current_skin > 0
        && let Some(skin) = skins.get(browser.current_skin)
        && let Err(err) = sk.set_skin(skin.clone())
    {
        warn!("spine_browser: initial set_skin({skin:?}) failed: {err:?}");
    }

    if let Some(anim) = anims.get(browser.current_animation).cloned() {
        sk.play(0, anim, true);
    }

    browser.animations = anims;
    browser.skins = skins;
}

fn live_fit_camera(
    time: Res<Time>,
    mut browser: ResMut<Browser>,
    sk_query: Query<&SpineSkeleton>,
    windows: Query<&Window>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if let Some(entity) = browser.skeleton_entity
        && let Ok(sk) = sk_query.get(entity)
        && let Some(state) = &sk.state
        && let Some((min, max)) = common::aggregate_bounds(state)
    {
        let center = (min + max) * 0.5;
        let height = ((max.y - min.y) * VIEW_MARGIN).max(MIN_VIEW_HEIGHT);
        // Grow the height enough to fit a wide rig horizontally.
        let aspect = windows
            .iter()
            .next()
            .map_or(16.0 / 9.0, |w| w.width() / w.height().max(1.0));
        let needed_for_width = (max.x - min.x) * VIEW_MARGIN / aspect;
        browser.target_view = View {
            center,
            height: height.max(needed_for_width).max(MIN_VIEW_HEIGHT),
        };
    }

    // Frame-rate independent easing; the clamp stops a long stall from snapping.
    let dt = time.delta_secs().clamp(0.0, 1.0 / 30.0);
    let alpha = 1.0 - (1.0 - VIEW_LERP).powf(dt * 60.0);
    browser.current_view.center = browser
        .current_view
        .center
        .lerp(browser.target_view.center, alpha);
    browser.current_view.height = lerp_f32(
        browser.current_view.height,
        browser.target_view.height,
        alpha,
    );

    let view = browser.current_view;
    if let Ok((mut transform, mut projection)) = camera_q.single_mut() {
        transform.translation.x = view.center.x;
        transform.translation.y = view.center.y;
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: view.height,
            };
            ortho.scale = 1.0;
        }
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn update_hud(browser: Res<Browser>, mut text_q: Query<&mut Text, With<HudText>>) {
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };
    let rig_label = &browser.rigs[browser.current_rig].label;
    let rig_count = browser.rigs.len();
    let anim_label = browser
        .animations
        .get(browser.current_animation)
        .map_or("(loading)", String::as_str);
    let skin_label = browser
        .skins
        .get(browser.current_skin)
        .map_or("(loading)", String::as_str);
    text.0 = format!(
        "rig {}/{}: {rig_label}\nanim {}/{}: {anim_label}\nskin {}/{}: {skin_label}\nspeed: {:.2}x\n\n[Space] rig  [Shift+Space] prev  [N] anim  [S] skin  [R] reset  [+/-] speed  [Esc] quit",
        browser.current_rig + 1,
        rig_count,
        browser.current_animation + 1,
        browser.animations.len(),
        browser.current_skin + 1,
        browser.skins.len(),
        browser.time_scale,
    );
}
