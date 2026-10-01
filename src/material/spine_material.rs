use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey};

use crate::material::shared::{SpineBlendMode, SpineColors, SpineMaterialKey};

/// 2D material emitted by [`crate::mesh`] for each batched `RenderCommand`
/// on entities tagged with [`crate::components::SpineRender2d`]. Pairs an
/// atlas-page texture with the slot's premultiplied colors and a blend
/// mode.
///
/// `blend_mode` is promoted into [`SpineMaterialKey`] so the
/// `Material2d` pipeline specializer caches one pipeline per mode (four
/// permutations total).
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
#[bind_group_data(SpineMaterialKey)]
pub struct SpineMaterial {
    /// Per-slot light + dark tint, shared across every vertex of one
    /// command (the runtime's adjacency batcher only merges commands
    /// with identical colors).
    #[uniform(0)]
    pub colors: SpineColors,
    /// Atlas page the command samples from. Resolved by the mesh-build
    /// system from `SpineAtlasAsset::pages` keyed by `RenderCommand::texture`.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    /// Spine blend mode. Not a bind-group field — copied into
    /// [`SpineMaterialKey`] for pipeline specialization.
    pub blend_mode: SpineBlendMode,
}

impl Default for SpineMaterial {
    fn default() -> Self {
        Self {
            colors: SpineColors::default(),
            texture: Handle::default(),
            blend_mode: SpineBlendMode::Normal,
        }
    }
}

impl From<&SpineMaterial> for SpineMaterialKey {
    fn from(m: &SpineMaterial) -> Self {
        Self {
            blend_mode: m.blend_mode,
        }
    }
}

const SHADER_ASSET_PATH: &str = "embedded://spine_bevy/material/spine.wgsl";

impl Material2d for SpineMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let blend = key.bind_group_data.blend_mode.blend_state();
        if let Some(fragment) = descriptor.fragment.as_mut()
            && let Some(Some(target)) = fragment.targets.first_mut().map(Option::as_mut)
        {
            target.blend = Some(blend);
        }
        Ok(())
    }
}

/// Register the 2D WGSL shader with Bevy's embedded asset source. Called by
/// [`crate::SpinePlugin`] during `build`. Downstream, `Material2d::fragment_shader`
/// returns the `embedded://...` path and the asset server resolves it
/// lazily — so this registration doesn't require any render plugins to be
/// installed yet.
pub(crate) fn register_spine_shader(app: &mut App) {
    embedded_asset!(app, "spine.wgsl");
}
