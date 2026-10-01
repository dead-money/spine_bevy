//! Asset types and loaders for Spine `.atlas`, `.skel` and `.json` files.

pub mod atlas_loader;
pub mod skel_json_loader;
pub mod skel_loader;

pub use atlas_loader::{SpineAtlasAsset, SpineAtlasLoader, SpineAtlasLoaderError};
pub use skel_json_loader::{
    SpineSkeletonJsonLoader, SpineSkeletonJsonLoaderError, SpineSkeletonJsonLoaderSettings,
};
pub use skel_loader::{
    SpineSkeletonAsset, SpineSkeletonLoader, SpineSkeletonLoaderError, SpineSkeletonLoaderSettings,
};

use bevy::asset::{AssetPath, ParseAssetPathError};
use thiserror::Error;

/// Returns `override_path` if given. Otherwise strips a `-pro`, `-ess` or
/// `-ios` suffix from the skeleton's file stem and resolves `<stem>.atlas`
/// beside it: `spineboy-pro.skel` gives `spineboy.atlas`.
pub(crate) fn derive_atlas_path(
    skel_path: &AssetPath<'static>,
    override_path: Option<&str>,
) -> Result<AssetPath<'static>, AtlasDeriveError> {
    if let Some(p) = override_path {
        return Ok(AssetPath::parse(p).clone_owned());
    }

    let stem = skel_path
        .path()
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AtlasDeriveError::BadStem(skel_path.to_string()))?;

    let base = ["-pro", "-ess", "-ios"]
        .into_iter()
        .find_map(|suffix| stem.strip_suffix(suffix))
        .unwrap_or(stem);

    let atlas_name = format!("{base}.atlas");
    Ok(skel_path.resolve_embed_str(&atlas_name)?)
}

#[derive(Debug, Error)]
pub(crate) enum AtlasDeriveError {
    #[error("could not derive atlas path from skeleton path {0:?}")]
    BadStem(String),
    #[error("asset path parse error: {0}")]
    Parse(#[from] ParseAssetPathError),
}
