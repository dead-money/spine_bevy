//! The [`SpineSkeleton`] component, its runtime state, and the render-mode
//! markers.

use bevy::prelude::*;

use spine_runtime::animation::{AnimationState, Event};
use spine_runtime::render::SkeletonRenderer;
use spine_runtime::skeleton::{Physics, Skeleton, SkinNotFound};

use crate::asset::SpineSkeletonAsset;
use crate::material::{SpineMaterial, SpineMaterial3d};

/// Render-mode marker: draw through the 2D `Material2d` pipeline. The
/// default; [`crate::systems::ensure_spine_render_marker`] inserts it on any
/// [`SpineSkeleton`] without a marker.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SpineRender2d;

/// Render-mode marker: draw through the 3D `Material` pipeline. Insert at
/// spawn alongside [`SpineSkeleton`]; don't combine with [`SpineRender2d`].
///
/// The rig lies in the entity's local XY plane; orient it with the entity's
/// `Transform`. Draw order comes from a small local-Z step per render
/// command, with depth writes off.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SpineRender3d;

/// One Spine skeleton instance. Holds the asset handle and, once the asset
/// has loaded, its runtime state ([`Skeleton`], [`AnimationState`],
/// [`SkeletonRenderer`]).
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct SpineSkeleton {
    /// Strong handle; keeps the asset loaded while the component exists.
    /// Replacing it after init keeps the old runtime state; spawn a new
    /// entity to switch assets.
    pub asset: Handle<SpineSkeletonAsset>,
    /// `None` until [`crate::systems::initialize_spine_skeletons`] runs after
    /// the asset loads.
    pub state: Option<SpineSkeletonState>,
    /// Multiplies `Time::delta_secs` for animation and physics. 0 freezes
    /// time but keeps applying and rendering the pose.
    pub time_scale: f32,
    /// Passed to [`Skeleton::update_world_transform`] each tick.
    pub physics: Physics,
    /// When `true`, the skeleton neither advances nor re-renders.
    pub paused: bool,
    /// Animation to start once the asset loads. Taken at init; after that,
    /// use [`SpineSkeleton::play`].
    pub pending_animation: Option<PendingAnimation>,
    /// Skin to activate once the asset loads, before the first tick. Taken
    /// at init; after that, use [`SpineSkeleton::set_skin`].
    pub pending_skin: Option<String>,
}

/// Per-instance runtime state of a loaded [`SpineSkeleton`].
pub struct SpineSkeletonState {
    pub skeleton: Skeleton,
    /// Use directly for queueing and mixing beyond [`SpineSkeleton::play`].
    pub animation_state: AnimationState,
    /// Holds the commands from the last tick; read with `renderer.commands()`.
    pub renderer: SkeletonRenderer,
    /// Keyframe events from the last tick. Drained by
    /// [`crate::systems::drain_spine_events`].
    pub events: Vec<Event>,
    /// One mesh per render-command index, grown to the largest command
    /// count seen. Index-parallel with `children` and the material vec in use.
    pub meshes: Vec<Handle<Mesh>>,
    /// Materials for [`SpineRender2d`] skeletons; empty otherwise.
    pub materials: Vec<Handle<SpineMaterial>>,
    /// Materials for [`SpineRender3d`] skeletons; empty otherwise.
    pub materials_3d: Vec<Handle<SpineMaterial3d>>,
    /// Child mesh entities. Ones past the current command count are hidden,
    /// not despawned, so they can be reused.
    pub children: Vec<Entity>,
}

/// Animation request held until the skeleton's asset loads.
#[derive(Clone, Debug)]
pub struct PendingAnimation {
    pub track: usize,
    pub name: String,
    pub looping: bool,
}

impl SpineSkeleton {
    /// `time_scale` 1.0, [`Physics::Update`], unpaused, nothing pending.
    #[must_use]
    pub fn new(asset: Handle<SpineSkeletonAsset>) -> Self {
        Self {
            asset,
            state: None,
            time_scale: 1.0,
            physics: Physics::Update,
            paused: false,
            pending_animation: None,
            pending_skin: None,
        }
    }

    /// Starts `name` on `track` once the asset loads.
    #[must_use]
    pub fn with_initial_animation(
        mut self,
        track: usize,
        name: impl Into<String>,
        looping: bool,
    ) -> Self {
        self.pending_animation = Some(PendingAnimation {
            track,
            name: name.into(),
            looping,
        });
        self
    }

    /// Sets `name` on `track` now if the asset has loaded, logging a warning
    /// if it fails. Otherwise replaces the pending animation.
    pub fn play(&mut self, track: usize, name: impl Into<String>, looping: bool) {
        let pending = PendingAnimation {
            track,
            name: name.into(),
            looping,
        };
        if let Some(state) = self.state.as_mut() {
            if let Err(err) = state.animation_state.set_animation_by_name(
                pending.track,
                &pending.name,
                pending.looping,
            ) {
                warn!(
                    "spine_bevy: set_animation_by_name({}, {:?}, {}) failed: {err:?}",
                    pending.track, pending.name, pending.looping
                );
            }
        } else {
            self.pending_animation = Some(pending);
        }
    }

    /// `None` until the asset has loaded.
    #[must_use]
    pub fn animation_state(&self) -> Option<&AnimationState> {
        self.state.as_ref().map(|s| &s.animation_state)
    }

    /// `None` until the asset has loaded.
    pub fn animation_state_mut(&mut self) -> Option<&mut AnimationState> {
        self.state.as_mut().map(|s| &mut s.animation_state)
    }

    /// `None` until the asset has loaded.
    #[must_use]
    pub fn skeleton(&self) -> Option<&Skeleton> {
        self.state.as_ref().map(|s| &s.skeleton)
    }

    /// `None` until the asset has loaded.
    pub fn skeleton_mut(&mut self) -> Option<&mut Skeleton> {
        self.state.as_mut().map(|s| &mut s.skeleton)
    }

    /// Activates skin `name` once the asset loads.
    #[must_use]
    pub fn with_initial_skin(mut self, name: impl Into<String>) -> Self {
        self.pending_skin = Some(name.into());
        self
    }

    /// Activates skin `name` and resets slots to the setup pose if the asset
    /// has loaded. Otherwise replaces the pending skin.
    ///
    /// # Errors
    /// [`SkinNotFound`] if the loaded data has no such skin. A queued skin
    /// returns `Ok(())`; an unknown name is logged at init.
    pub fn set_skin(&mut self, name: impl Into<String>) -> Result<(), SkinNotFound> {
        let name = name.into();
        if let Some(state) = self.state.as_mut() {
            state.skeleton.set_skin_by_name(&name)?;
            state.skeleton.setup_pose_slots();
            Ok(())
        } else {
            self.pending_skin = Some(name);
            Ok(())
        }
    }

    /// Animation names in data order. `None` until the asset has loaded.
    #[must_use]
    pub fn available_animations(&self) -> Option<Vec<&str>> {
        let skel = self.skeleton()?;
        Some(
            skel.data()
                .animations
                .iter()
                .map(|a| a.name.as_str())
                .collect(),
        )
    }

    /// Skin names in data order, including `default` when the data has one.
    /// `None` until the asset has loaded.
    #[must_use]
    pub fn available_skins(&self) -> Option<Vec<&str>> {
        let skel = self.skeleton()?;
        Some(skel.data().skins.iter().map(|s| s.name.as_str()).collect())
    }
}
