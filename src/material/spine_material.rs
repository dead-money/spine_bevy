use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey};

use crate::material::shared::{SpineBlendMode, SpineColors, SpineMaterialKey};

/// 2D material for one render command of a
/// [`SpineRender2d`](crate::SpineRender2d) skeleton. Expects a
/// premultiplied-alpha atlas page.
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
#[bind_group_data(SpineMaterialKey)]
pub struct SpineMaterial {
    /// The command's colors. Uniform per command because the runtime's
    /// default render options only batch slots with identical colors.
    #[uniform(0)]
    pub colors: SpineColors,
    /// The command's atlas page, from `SpineAtlasAsset::pages`.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    /// Selects the pipeline through [`SpineMaterialKey`]; not bound.
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

/// Embeds `spine.wgsl`. Needs no render plugins installed yet.
pub(crate) fn register_spine_shader(app: &mut App) {
    embedded_asset!(app, "spine.wgsl");
}
