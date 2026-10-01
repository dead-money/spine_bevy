//! 3D (`Material`) version of the Spine material. Unlit: Spine colors are
//! authored tints, not surfaces to light.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::material::shared::{SpineBlendMode, SpineColors, SpineMaterialKey};

/// 3D material for one render command of a
/// [`SpineRender3d`](crate::SpineRender3d) skeleton. Same fields and bindings
/// as [`SpineMaterial`](crate::SpineMaterial).
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
#[bind_group_data(SpineMaterialKey)]
pub struct SpineMaterial3d {
    #[uniform(0)]
    pub colors: SpineColors,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    pub blend_mode: SpineBlendMode,
}

impl Default for SpineMaterial3d {
    fn default() -> Self {
        Self {
            colors: SpineColors::default(),
            texture: Handle::default(),
            blend_mode: SpineBlendMode::Normal,
        }
    }
}

impl From<&SpineMaterial3d> for SpineMaterialKey {
    fn from(m: &SpineMaterial3d) -> Self {
        Self {
            blend_mode: m.blend_mode,
        }
    }
}

const SHADER_ASSET_PATH: &str = "embedded://spine_bevy/material/spine_3d.wgsl";

impl Material for SpineMaterial3d {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    // Translucent geometry: keep it out of the depth prepass and shadows.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let blend = key.bind_group_data.blend_mode.blend_state();
        if let Some(fragment) = descriptor.fragment.as_mut()
            && let Some(Some(target)) = fragment.targets.first_mut().map(Option::as_mut)
        {
            target.blend = Some(blend);
        }
        // Commands sit only `Z_OFFSET_PER_COMMAND` apart; without depth
        // writes, draw order comes from the transparent sort alone.
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = Some(false);
        }
        // Winding flips with negative scale, and the rig should be visible
        // from behind, as in 2D.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Embeds `spine_3d.wgsl`.
pub(crate) fn register_spine_shader_3d(app: &mut App) {
    embedded_asset!(app, "spine_3d.wgsl");
}
