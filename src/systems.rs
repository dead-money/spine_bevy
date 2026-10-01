use std::sync::Arc;

use bevy::prelude::*;

use spine_runtime::animation::{
    AnimationState, AnimationStateData, Event as SpineEvent, state::StateEvent,
};
use spine_runtime::render::SkeletonRenderer;
use spine_runtime::skeleton::{Physics, Skeleton};

use crate::asset::SpineSkeletonAsset;
use crate::components::{SpineRender2d, SpineRender3d, SpineSkeleton, SpineSkeletonState};

/// The plugin's system sets, chained in this order in `Update`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpineSet {
    /// Inserts [`SpineRender2d`] on skeletons with no render-mode marker.
    EnsureMarkers,
    /// Builds [`SpineSkeletonState`] once the asset has loaded.
    Init,
    /// Advances and applies animations, updates world transforms, renders.
    Tick,
    /// Writes the frame's render commands into meshes and materials.
    BuildMeshes,
    /// Forwards events as [`SpineStateEvent`] and [`SpineKeyframeEvent`]
    /// messages.
    Events,
}

/// Inserted by [`initialize_spine_skeletons`] once a skeleton's runtime state
/// exists.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpineInitialized;

/// Builds the [`Skeleton`] and [`AnimationState`] for each uninitialized
/// [`SpineSkeleton`] whose asset has loaded, applies the pending animation
/// and skin (logging failures), and inserts [`SpineInitialized`].
pub fn initialize_spine_skeletons(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpineSkeleton), Without<SpineInitialized>>,
    assets: Res<Assets<SpineSkeletonAsset>>,
) {
    for (entity, mut sk) in &mut query {
        let Some(asset) = assets.get(&sk.asset) else {
            continue;
        };

        let data = Arc::clone(&asset.data);
        let mut skeleton = Skeleton::new(Arc::clone(&data));
        skeleton.update_cache();
        skeleton.setup_pose();
        skeleton.update_world_transform(Physics::None);

        let state_data = Arc::new(AnimationStateData::new(Arc::clone(&data)));
        let mut animation_state = AnimationState::new(state_data);

        if let Some(pending) = sk.pending_animation.take()
            && let Err(err) =
                animation_state.set_animation_by_name(pending.track, &pending.name, pending.looping)
        {
            warn!(
                "spine_bevy: pending animation {:?} on track {} failed: {err:?}",
                pending.name, pending.track
            );
        }

        if let Some(skin) = sk.pending_skin.take() {
            if let Err(err) = skeleton.set_skin_by_name(&skin) {
                warn!("spine_bevy: pending skin {skin:?} failed: {err:?}");
            } else {
                skeleton.setup_pose_slots();
            }
        }

        sk.state = Some(SpineSkeletonState {
            skeleton,
            animation_state,
            renderer: SkeletonRenderer::new(),
            events: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            materials_3d: Vec::new(),
            children: Vec::new(),
        });
        commands.entity(entity).insert(SpineInitialized);
    }
}

/// Inserts [`SpineRender2d`] on every [`SpineSkeleton`] that has neither
/// render-mode marker, making 2D the default.
#[allow(clippy::type_complexity)]
pub fn ensure_spine_render_marker(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<SpineSkeleton>,
            Without<SpineRender2d>,
            Without<SpineRender3d>,
        ),
    >,
) {
    for entity in &query {
        commands.entity(entity).insert(SpineRender2d);
    }
}

/// Advances each unpaused, initialized skeleton by `delta_secs * time_scale`:
/// updates and applies its [`AnimationState`], collects keyframe events,
/// updates world transforms with the component's [`Physics`] mode, and
/// renders into its [`SkeletonRenderer`]. Runs in parallel over skeletons.
pub fn tick_spine_skeletons(
    time: Res<Time>,
    mut query: Query<&mut SpineSkeleton, With<SpineInitialized>>,
) {
    let base_dt = time.delta_secs();
    query.par_iter_mut().for_each(|mut sk| {
        if sk.paused {
            return;
        }
        let scale = sk.time_scale;
        let physics = sk.physics;
        let Some(state) = sk.state.as_mut() else {
            return;
        };

        let dt = base_dt * scale;
        state.animation_state.update(dt);
        state.events.clear();
        state
            .animation_state
            .apply(&mut state.skeleton, &mut state.events);
        state.skeleton.update(dt);
        state.skeleton.update_world_transform(physics);
        let _ = state.renderer.render(&state.skeleton);
    });
}

/// A track-entry lifecycle event, or a keyframe event wrapped as
/// `EventType::Event`, from the skeleton on `entity`. Read with a
/// `MessageReader`.
#[derive(Message, Debug, Clone)]
pub struct SpineStateEvent {
    pub entity: Entity,
    pub event: StateEvent,
}

/// A keyframe event from the skeleton on `entity`. The same event also
/// arrives as a [`SpineStateEvent`] with `EventType::Event`; this message is
/// for readers that want only keyframes.
#[derive(Message, Debug, Clone)]
pub struct SpineKeyframeEvent {
    pub entity: Entity,
    pub event: SpineEvent,
}

/// Drains each skeleton's state and keyframe events into
/// [`SpineStateEvent`] and [`SpineKeyframeEvent`] messages.
pub fn drain_spine_events(
    mut query: Query<(Entity, &mut SpineSkeleton)>,
    mut state_writer: MessageWriter<SpineStateEvent>,
    mut keyframe_writer: MessageWriter<SpineKeyframeEvent>,
) {
    for (entity, mut sk) in &mut query {
        let Some(state) = sk.state.as_mut() else {
            continue;
        };
        for event in state.animation_state.drain_events() {
            state_writer.write(SpineStateEvent {
                entity,
                event: event.clone(),
            });
        }
        for event in state.events.drain(..) {
            keyframe_writer.write(SpineKeyframeEvent { entity, event });
        }
    }
}
