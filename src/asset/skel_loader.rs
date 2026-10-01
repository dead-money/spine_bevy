//! The binary `.skel` skeleton loader.

use std::sync::Arc;

use bevy::asset::{AssetLoader, AssetPath, LoadContext, io::Reader};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use spine_runtime::data::SkeletonData;
use spine_runtime::load::{AtlasAttachmentLoader, BinaryError, SkeletonBinary};

use crate::asset::atlas_loader::SpineAtlasAsset;

/// Loaded skeleton data, shared by every [`SpineSkeleton`](crate::SpineSkeleton)
/// spawned from it, and the atlas it was loaded against. Produced by both
/// [`SpineSkeletonLoader`] and
/// [`SpineSkeletonJsonLoader`](crate::SpineSkeletonJsonLoader).
#[derive(Asset, TypePath, Debug)]
pub struct SpineSkeletonAsset {
    pub data: Arc<SkeletonData>,
    /// Supplies the page images at draw time.
    pub atlas: Handle<SpineAtlasAsset>,
}

/// Per-load settings for [`SpineSkeletonLoader`].
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct SpineSkeletonLoaderSettings {
    /// Atlas asset path from the asset root, or `None` to derive it from the
    /// skeleton path: strip a `-pro`, `-ess` or `-ios` suffix from the stem
    /// and append `.atlas` (`spineboy-pro.skel` gives `spineboy.atlas`).
    pub atlas_path: Option<String>,
    /// Load-time scale for positions and sizes. `None` means 1.0.
    pub scale: Option<f32>,
}

/// Loads binary `.skel` skeletons and their atlas into a
/// [`SpineSkeletonAsset`].
#[derive(Default, TypePath)]
pub struct SpineSkeletonLoader;

/// Why [`SpineSkeletonLoader`] failed.
#[derive(Debug, Error)]
pub enum SpineSkeletonLoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The skeleton path has no UTF-8 file stem to derive an atlas name from.
    #[error("could not derive atlas path from skeleton path {0:?}")]
    AtlasPathDerivation(String),
    /// The atlas at the given path failed to load; the second field is the
    /// asset server's error message.
    #[error("failed to load companion atlas {0:?}: {1}")]
    AtlasLoad(String, String),
    #[error("binary skeleton parse error: {0}")]
    Parse(#[from] BinaryError),
    /// The derived atlas path is not a valid asset path.
    #[error("asset path parse error: {0}")]
    Path(#[from] bevy::asset::ParseAssetPathError),
}

impl From<super::AtlasDeriveError> for SpineSkeletonLoaderError {
    fn from(e: super::AtlasDeriveError) -> Self {
        match e {
            super::AtlasDeriveError::BadStem(s) => Self::AtlasPathDerivation(s),
            super::AtlasDeriveError::Parse(e) => Self::Path(e),
        }
    }
}

impl AssetLoader for SpineSkeletonLoader {
    type Asset = SpineSkeletonAsset;
    type Settings = SpineSkeletonLoaderSettings;
    type Error = SpineSkeletonLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let atlas_path = resolve_atlas_path(load_context.path(), settings.atlas_path.as_deref())?;

        // The handle keeps the atlas (and its page images) in the asset server
        // for drawing; `load_value` gives the parsed atlas needed to resolve
        // attachments now.
        let atlas_handle: Handle<SpineAtlasAsset> = load_context.load(atlas_path.clone());

        let loaded_atlas = load_context
            .load_builder()
            .load_value::<SpineAtlasAsset>(atlas_path.clone())
            .await
            .map_err(|e| {
                SpineSkeletonLoaderError::AtlasLoad(atlas_path.to_string(), e.to_string())
            })?;
        let atlas_asset = loaded_atlas.get();
        let mut attachment_loader = AtlasAttachmentLoader::new(&atlas_asset.atlas);

        let mut binary = SkeletonBinary::with_loader(&mut attachment_loader);
        if let Some(scale) = settings.scale {
            binary = binary.with_scale(scale);
        }
        let data = binary.read(&bytes)?;

        Ok(SpineSkeletonAsset {
            data: Arc::new(data),
            atlas: atlas_handle,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["skel"]
    }
}

fn resolve_atlas_path(
    skel_path: &AssetPath<'static>,
    override_path: Option<&str>,
) -> Result<AssetPath<'static>, SpineSkeletonLoaderError> {
    super::derive_atlas_path(skel_path, override_path).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_path(p: &str) -> AssetPath<'static> {
        AssetPath::from(PathBuf::from(p))
    }

    #[test]
    fn strips_pro_suffix() {
        let skel = make_path("rigs/spineboy/export/spineboy-pro.skel");
        let atlas = resolve_atlas_path(&skel, None).unwrap();
        assert_eq!(
            atlas.path().to_str(),
            Some("rigs/spineboy/export/spineboy.atlas")
        );
    }

    #[test]
    fn strips_ess_suffix() {
        let skel = make_path("rigs/spineboy-ess.skel");
        let atlas = resolve_atlas_path(&skel, None).unwrap();
        assert_eq!(atlas.path().to_str(), Some("rigs/spineboy.atlas"));
    }

    #[test]
    fn keeps_unsuffixed_stem() {
        let skel = make_path("rigs/raptor.skel");
        let atlas = resolve_atlas_path(&skel, None).unwrap();
        assert_eq!(atlas.path().to_str(), Some("rigs/raptor.atlas"));
    }

    #[test]
    fn honours_override() {
        let skel = make_path("rigs/spineboy-pro.skel");
        let atlas = resolve_atlas_path(&skel, Some("packs/hero.atlas")).unwrap();
        assert_eq!(atlas.path().to_str(), Some("packs/hero.atlas"));
    }
}
