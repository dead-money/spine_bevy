//! Uniform layout, blend states and pipeline key shared by the 2D and 3D
//! materials.

use bevy::prelude::*;
use bevy::render::render_resource::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ShaderType,
};

use spine_runtime::data::BlendMode;

/// A render command's light and dark (tint-black) colors. Must match
/// `SpineColors` in both WGSL shaders, at binding 0 of the material group.
#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct SpineColors {
    /// `RenderCommand::colors`, RGB premultiplied by alpha.
    pub light: Vec4,
    /// `RenderCommand::dark_colors`, RGB premultiplied by the light alpha;
    /// black when the slot has no dark color. Alpha is always 1.
    pub dark: Vec4,
}

/// [`BlendMode`] as a pipeline-key value.
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
    /// Blend state for premultiplied-alpha output.
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

/// Pipeline specialization key for both materials: one pipeline per blend
/// mode.
#[repr(C)]
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SpineMaterialKey {
    pub blend_mode: SpineBlendMode,
}
