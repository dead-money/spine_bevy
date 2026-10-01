//! Render-backend-agnostic pieces reused by both the 2D (`Material2d`) and
//! 3D (`Material`) flavors of the Spine material. The blend-state tables,
//! color uniform layout, and per-pipeline specialization key are identical
//! in both pipelines; only the trait implementations and shader imports
//! differ.

use bevy::prelude::*;
use bevy::render::render_resource::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ShaderType,
};

use spine_runtime::data::BlendMode;

/// Per-material uniform carrying the slot's light + dark tint as
/// premultiplied RGBA. Matches the `SpineColors` struct in the shaders at
/// `@group(2) @binding(0)`.
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct SpineColors {
    /// Light tint, premultiplied-alpha. The runtime produces this
    /// premultiplied already (color packing in `RenderCommand::colors`).
    pub light: Vec4,
    /// Tint-black (`darkColor` in spine-cpp). Alpha byte is always `0xff`
    /// on the CPU side; the shader never reads `dark.a`.
    pub dark: Vec4,
}

/// Bevy-side mirror of `spine_runtime::data::BlendMode`. Lives in the
/// plugin crate so the runtime crate doesn't take a `bevy` dep.
#[repr(u8)]
#[derive(Copy, Clone, Hash, Eq, PartialEq, Default, Debug)]
pub enum SpineBlendMode {
    #[default]
    Normal,
    Additive,
    Multiply,
    Screen,
}

impl From<BlendMode> for SpineBlendMode {
    fn from(mode: BlendMode) -> Self {
        match mode {
            BlendMode::Normal => Self::Normal,
            BlendMode::Additive => Self::Additive,
            BlendMode::Multiply => Self::Multiply,
            BlendMode::Screen => Self::Screen,
        }
    }
}

impl SpineBlendMode {
    /// wgpu blend-state for PMA atlases, by Spine blend mode.
    ///
    /// All four modes use `BlendOperation::Add` for both color and alpha.
    #[must_use]
    pub fn blend_state(self) -> BlendState {
        let color = match self {
            Self::Normal => BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            Self::Additive => BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            Self::Multiply => BlendComponent {
                src_factor: BlendFactor::Dst,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            Self::Screen => BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrc,
                operation: BlendOperation::Add,
            },
        };
        let alpha = match self {
            Self::Normal | Self::Additive => color,
            Self::Multiply => BlendComponent {
                src_factor: BlendFactor::OneMinusSrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            Self::Screen => BlendComponent {
                src_factor: BlendFactor::OneMinusSrc,
                dst_factor: BlendFactor::OneMinusSrc,
                operation: BlendOperation::Add,
            },
        };
        BlendState { color, alpha }
    }
}

/// Specialization key shared by the 2D and 3D materials: one pipeline per
/// blend mode. The Material2d and Material trait impls both promote this
/// via `#[bind_group_data(SpineMaterialKey)]`.
#[repr(C)]
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SpineMaterialKey {
    pub blend_mode: SpineBlendMode,
}
