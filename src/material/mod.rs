//! The materials that draw Spine render commands: [`SpineMaterial`] for 2D,
//! [`SpineMaterial3d`] for 3D. Both apply Spine's two-color tint to a
//! premultiplied-alpha atlas page and pick a pipeline per [`SpineBlendMode`].

pub mod shared;
pub mod spine_material;
pub mod spine_material_3d;

pub use shared::{SpineBlendMode, SpineColors, SpineMaterialKey};
pub use spine_material::SpineMaterial;
pub use spine_material_3d::SpineMaterial3d;
