use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh3d, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use spine_runtime::render::RenderCommand;

use crate::asset::SpineAtlasAsset;
use crate::components::{SpineRender2d, SpineRender3d, SpineSkeleton, SpineSkeletonState};
use crate::material::{SpineBlendMode, SpineColors, SpineMaterial, SpineMaterial3d};

/// Local Z step per render command, so the transparent phases (sorted by z in
/// 2D, by camera distance in 3D) keep the runtime's back-to-front order.
const Z_OFFSET_PER_COMMAND: f32 = 0.001;

/// Writes each [`SpineRender2d`] skeleton's render commands into one child
/// `Mesh2d` + [`SpineMaterial`] per command. Children are spawned only when
/// the command count grows; extras are hidden. Skips skeletons whose asset
/// or atlas isn't loaded.
pub fn build_spine_meshes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpineSkeleton), With<SpineRender2d>>,
    atlases: Res<Assets<SpineAtlasAsset>>,
    skeleton_assets: Res<Assets<crate::asset::SpineSkeletonAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpineMaterial>>,
    mut child_vis: Query<&mut Visibility>,
) {
    for (entity, mut sk) in &mut query {
        let Some(skel_asset) = skeleton_assets.get(&sk.asset) else {
            continue;
        };
        let Some(atlas) = atlases.get(&skel_asset.atlas) else {
            continue;
        };
        let Some(state) = sk.state.as_mut() else {
            continue;
        };

        let cmd_count = state.renderer.commands().len();
        grow_child_buffers_2d(
            &mut commands,
            entity,
            state,
            cmd_count,
            &mut meshes,
            &mut materials,
        );

        for i in 0..cmd_count {
            let cmd = &state.renderer.commands()[i];
            let tex = atlas
                .pages
                .get(cmd.texture.0 as usize)
                .cloned()
                .unwrap_or_default();

            if let Some(mesh) = meshes.get_mut(&state.meshes[i]) {
                write_mesh_from_command(mesh.into_inner(), cmd);
            }
            if let Some(mut mat) = materials.get_mut(&state.materials[i]) {
                mat.texture = tex;
                mat.colors = colors_from_command(cmd);
                mat.blend_mode = SpineBlendMode::from(cmd.blend_mode);
            }
            if let Ok(mut vis) = child_vis.get_mut(state.children[i]) {
                *vis = Visibility::Visible;
            }
        }

        for &child in &state.children[cmd_count..] {
            if let Ok(mut vis) = child_vis.get_mut(child) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// [`build_spine_meshes`] for [`SpineRender3d`] skeletons: children carry
/// `Mesh3d` + [`SpineMaterial3d`]. Vertices are `(x, y, 0)` in the entity's
/// local space.
pub fn build_spine_meshes_3d(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpineSkeleton), With<SpineRender3d>>,
    atlases: Res<Assets<SpineAtlasAsset>>,
    skeleton_assets: Res<Assets<crate::asset::SpineSkeletonAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpineMaterial3d>>,
    mut child_vis: Query<&mut Visibility>,
) {
    for (entity, mut sk) in &mut query {
        let Some(skel_asset) = skeleton_assets.get(&sk.asset) else {
            continue;
        };
        let Some(atlas) = atlases.get(&skel_asset.atlas) else {
            continue;
        };
        let Some(state) = sk.state.as_mut() else {
            continue;
        };

        let cmd_count = state.renderer.commands().len();
        grow_child_buffers_3d(
            &mut commands,
            entity,
            state,
            cmd_count,
            &mut meshes,
            &mut materials,
        );

        for i in 0..cmd_count {
            let cmd = &state.renderer.commands()[i];
            let tex = atlas
                .pages
                .get(cmd.texture.0 as usize)
                .cloned()
                .unwrap_or_default();

            if let Some(mesh) = meshes.get_mut(&state.meshes[i]) {
                write_mesh_from_command(mesh.into_inner(), cmd);
            }
            if let Some(mut mat) = materials.get_mut(&state.materials_3d[i]) {
                mat.texture = tex;
                mat.colors = colors_from_command(cmd);
                mat.blend_mode = SpineBlendMode::from(cmd.blend_mode);
            }
            if let Ok(mut vis) = child_vis.get_mut(state.children[i]) {
                *vis = Visibility::Visible;
            }
        }

        for &child in &state.children[cmd_count..] {
            if let Ok(mut vis) = child_vis.get_mut(child) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

fn grow_child_buffers_2d(
    commands: &mut Commands,
    parent: Entity,
    state: &mut SpineSkeletonState,
    cmd_count: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<SpineMaterial>,
) {
    while state.meshes.len() < cmd_count {
        let i = state.meshes.len();
        let mesh_handle = meshes.add(empty_mesh());
        let material_handle = materials.add(SpineMaterial::default());
        let z = (i as f32) * Z_OFFSET_PER_COMMAND;
        let child = commands
            .spawn((
                Mesh2d(mesh_handle.clone()),
                MeshMaterial2d(material_handle.clone()),
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::Hidden,
                ChildOf(parent),
            ))
            .id();
        state.meshes.push(mesh_handle);
        state.materials.push(material_handle);
        state.children.push(child);
    }
}

fn grow_child_buffers_3d(
    commands: &mut Commands,
    parent: Entity,
    state: &mut SpineSkeletonState,
    cmd_count: usize,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<SpineMaterial3d>,
) {
    while state.meshes.len() < cmd_count {
        let i = state.meshes.len();
        let mesh_handle = meshes.add(empty_mesh());
        let material_handle = materials.add(SpineMaterial3d::default());
        let z = (i as f32) * Z_OFFSET_PER_COMMAND;
        let child = commands
            .spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_handle.clone()),
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::Hidden,
                ChildOf(parent),
            ))
            .id();
        state.meshes.push(mesh_handle);
        state.materials_3d.push(material_handle);
        state.children.push(child);
    }
}

fn empty_mesh() -> Mesh {
    // Must stay in main world; `RENDER_WORLD` alone drops the mesh after
    // extract and the next frame's `insert_attribute` then panics.
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
}

fn colors_from_command(cmd: &RenderCommand) -> SpineColors {
    SpineColors {
        light: unpack_argb(cmd.colors.first().copied().unwrap_or(0xffff_ffff)),
        dark: unpack_argb(cmd.dark_colors.first().copied().unwrap_or(0xff00_0000)),
    }
}

/// Copies a command's positions, UVs and indices into `mesh`, reusing its
/// attribute buffers after the first call.
pub(crate) fn write_mesh_from_command(mesh: &mut Mesh, cmd: &RenderCommand) {
    let n = cmd.num_vertices();

    if let Some(VertexAttributeValues::Float32x3(buf)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        buf.clear();
        buf.extend((0..n).map(|i| [cmd.positions[i * 2], cmd.positions[i * 2 + 1], 0.0]));
    } else {
        let positions: Vec<[f32; 3]> = (0..n)
            .map(|i| [cmd.positions[i * 2], cmd.positions[i * 2 + 1], 0.0])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    }

    if let Some(VertexAttributeValues::Float32x2(buf)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        buf.clear();
        buf.extend((0..n).map(|i| [cmd.uvs[i * 2], cmd.uvs[i * 2 + 1]]));
    } else {
        let uvs: Vec<[f32; 2]> = (0..n)
            .map(|i| [cmd.uvs[i * 2], cmd.uvs[i * 2 + 1]])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }

    if let Some(Indices::U16(buf)) = mesh.indices_mut() {
        buf.clear();
        buf.extend_from_slice(&cmd.indices);
    } else {
        mesh.insert_indices(Indices::U16(cmd.indices.clone()));
    }
}

/// Unpacks `0xAARRGGBB` into RGBA in `0..=1`. No alpha premultiplication.
pub(crate) fn unpack_argb(v: u32) -> Vec4 {
    let a = ((v >> 24) & 0xff) as f32 / 255.0;
    let r = ((v >> 16) & 0xff) as f32 / 255.0;
    let g = ((v >> 8) & 0xff) as f32 / 255.0;
    let b = (v & 0xff) as f32 / 255.0;
    Vec4::new(r, g, b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_white_opaque() {
        assert_eq!(unpack_argb(0xffff_ffff), Vec4::splat(1.0));
    }

    #[test]
    fn unpacks_red_half_alpha_matches_runtime_pack() {
        // The runtime packs alpha 0.5 as 0x7f (truncated, like spine-cpp).
        let v = unpack_argb(0x7fff_0000);
        assert!((v.x - 1.0).abs() < 1e-6);
        assert!(v.y.abs() < 1e-6);
        assert!(v.z.abs() < 1e-6);
        assert!((v.w - 127.0 / 255.0).abs() < 1e-6);
    }
}
