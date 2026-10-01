//! Helpers shared by the examples, pulled in with `mod common;`: asset-root
//! resolution, rig discovery, render-command bounds, and a screenshot driver.

#![allow(dead_code)] // Each example uses a subset of the helpers.

use std::path::{Path, PathBuf};

use bevy::math::Vec2;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use spine_bevy::SpineSkeletonState;

/// One skeleton export and its atlas. Paths are relative to the asset root,
/// ready to pass to the `AssetServer`.
#[derive(Clone, Debug)]
pub struct RigEntry {
    pub label: String,
    pub skel_relpath: String,
    pub atlas_relpath: String,
}

/// Resolves the asset root from `cli_assets`, then `SPINE_EXAMPLES_DIR`, then
/// `../spine-runtimes/examples` or `./spine-runtimes/examples`.
///
/// The path is canonicalized because `AssetPlugin` resolves a relative
/// `file_path` against its own base directory, not the current directory.
/// The error message has no program-name prefix; callers add one.
pub fn resolve_asset_root(cli_assets: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = cli_assets {
        return validate_root(p);
    }
    if let Ok(p) = std::env::var("SPINE_EXAMPLES_DIR") {
        return validate_root(PathBuf::from(p));
    }
    for fallback in ["../spine-runtimes/examples", "./spine-runtimes/examples"] {
        let p = PathBuf::from(fallback);
        if p.is_dir() {
            return validate_root(p);
        }
    }
    Err(MISSING_ASSETS_HELP.to_string())
}

fn validate_root(p: PathBuf) -> Result<PathBuf, String> {
    if !p.is_dir() {
        return Err(format!(
            "asset path {} does not exist or is not a directory",
            p.display()
        ));
    }
    p.canonicalize()
        .map_err(|e| format!("cannot canonicalize asset path {}: {e}", p.display()))
}

const MISSING_ASSETS_HELP: &str = "Could not find Spine example rigs.

Pass an asset root via one of:
  --assets <path>
  SPINE_EXAMPLES_DIR=<path>
  ./spine-runtimes/examples
  ../spine-runtimes/examples

The expected source is the upstream spine-runtimes repo:
  git clone https://github.com/EsotericSoftware/spine-runtimes ../spine-runtimes

(Spine example art is licensed separately from this crate and is not bundled.)";

/// Returns one [`RigEntry`] per `<root>/<rig>/export/*.skel` that has a
/// matching atlas, sorted by label.
pub fn discover_rigs(root: &Path) -> Vec<RigEntry> {
    let mut out = Vec::new();
    let Ok(rig_dirs) = std::fs::read_dir(root) else {
        return out;
    };
    for rig_dir in rig_dirs.flatten() {
        let rig_dir = rig_dir.path();
        if !rig_dir.is_dir() {
            continue;
        }
        let export = rig_dir.join("export");
        if !export.is_dir() {
            continue;
        }
        let rig_name = rig_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let skels: Vec<PathBuf> = std::fs::read_dir(&export)
            .map(|it| {
                it.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "skel"))
                    .collect()
            })
            .unwrap_or_default();
        for skel in skels {
            let stem = skel
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            let Some(atlas) = pick_atlas(&export, &stem) else {
                continue;
            };
            out.push(RigEntry {
                label: format!("{rig_name} / {stem}"),
                skel_relpath: relpath(root, &skel),
                atlas_relpath: relpath(root, &atlas),
            });
        }
    }
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

/// Tries `<base>-pma.atlas`, then `<base>.atlas`, where `<base>` is the stem
/// without a trailing `-pro`, `-ess`, or `-ios`.
fn pick_atlas(export: &Path, skel_stem: &str) -> Option<PathBuf> {
    let base = ["-pro", "-ess", "-ios"]
        .into_iter()
        .find_map(|sfx| skel_stem.strip_suffix(sfx))
        .unwrap_or(skel_stem);
    for candidate in [format!("{base}-pma.atlas"), format!("{base}.atlas")] {
        let p = export.join(candidate);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn relpath(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `(min, max)` of the skeleton's latest render commands, or `None` when no
/// command has geometry.
pub fn aggregate_bounds(state: &SpineSkeletonState) -> Option<(Vec2, Vec2)> {
    let mut have_any = false;
    let mut xmin = f32::INFINITY;
    let mut xmax = f32::NEG_INFINITY;
    let mut ymin = f32::INFINITY;
    let mut ymax = f32::NEG_INFINITY;
    for cmd in state.renderer.commands() {
        if let Some((cxmin, cxmax, cymin, cymax)) = cmd.position_bounds() {
            xmin = xmin.min(cxmin);
            xmax = xmax.max(cxmax);
            ymin = ymin.min(cymin);
            ymax = ymax.max(cymax);
            have_any = true;
        }
    }
    have_any.then_some((Vec2::new(xmin, ymin), Vec2::new(xmax, ymax)))
}

#[derive(Resource)]
struct ScreenshotState {
    path: String,
    trigger: u32,
    current: u32,
    taken: bool,
}

/// Output path and trigger frame for [`install_screenshot_driver`].
pub struct ScreenshotConfig {
    pub path: String,
    pub trigger_frame: u32,
}

impl ScreenshotConfig {
    /// Returns `None` unless the `env_path` variable is set. A missing or
    /// unparseable `env_frames` falls back to `default_frames`.
    pub fn from_env(env_path: &str, env_frames: &str, default_frames: u32) -> Option<Self> {
        let path = std::env::var(env_path).ok()?;
        let trigger_frame = std::env::var(env_frames)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default_frames);
        Some(Self {
            path,
            trigger_frame,
        })
    }
}

/// Saves one window screenshot to `cfg.path` at frame `cfg.trigger_frame`,
/// then exits 30 frames later so the async PNG write can finish.
pub fn install_screenshot_driver(app: &mut App, cfg: ScreenshotConfig) {
    app.insert_resource(ScreenshotState {
        path: cfg.path,
        trigger: cfg.trigger_frame,
        current: 0,
        taken: false,
    });
    app.add_systems(Update, screenshot_driver_system);
}

fn screenshot_driver_system(
    mut commands: Commands,
    mut state: ResMut<ScreenshotState>,
    mut exit: MessageWriter<AppExit>,
) {
    state.current += 1;
    const GRACE: u32 = 30;
    if state.taken {
        if state.current >= state.trigger + GRACE {
            exit.write(AppExit::Success);
        }
        return;
    }
    if state.current >= state.trigger {
        let path = state.path.clone();
        info!("spine_bevy example: screenshot to {path}");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        state.taken = true;
    }
}
