//! The JSON `.json` skeleton loader.

use std::sync::Arc;

use bevy::asset::{AssetLoader, AssetPath, LoadContext, io::Reader};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use spine_runtime::load::{AtlasAttachmentLoader, JsonError, SkeletonJson};

use crate::asset::atlas_loader::SpineAtlasAsset;
use crate::asset::skel_loader::SpineSkeletonAsset;

/// Per-load settings for [`SpineSkeletonJsonLoader`]. Same fields and atlas
/// derivation as [`SpineSkeletonLoaderSettings`](crate::SpineSkeletonLoaderSettings).
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct SpineSkeletonJsonLoaderSettings {
    /// Atlas asset path from the asset root, or `None` to derive it from the
    /// skeleton path (`spineboy-pro.json` gives `spineboy.atlas`).
    pub atlas_path: Option<String>,
    /// Load-time scale for positions and sizes. `None` means 1.0.
    pub scale: Option<f32>,
}

/// Loads `.json` skeletons and their atlas into a [`SpineSkeletonAsset`].
#[derive(Default, TypePath)]
pub struct SpineSkeletonJsonLoader;

/// Why [`SpineSkeletonJsonLoader`] failed.
#[derive(Debug, Error)]
pub enum SpineSkeletonJsonLoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The skeleton path has no UTF-8 file stem to derive an atlas name from.
    #[error("could not derive atlas path from skeleton path {0:?}")]
    AtlasPathDerivation(String),
    /// The atlas at the given path failed to load; the second field is the
    /// asset server's error message.
    #[error("failed to load companion atlas {0:?}: {1}")]
    AtlasLoad(String, String),
    #[error("json skeleton parse error: {0}")]
    Parse(#[from] JsonError),
    /// The derived atlas path is not a valid asset path.
    #[error("asset path parse error: {0}")]
    Path(#[from] bevy::asset::ParseAssetPathError),
}

impl From<super::AtlasDeriveError> for SpineSkeletonJsonLoaderError {
    fn from(e: super::AtlasDeriveError) -> Self {
        match e {
            super::AtlasDeriveError::BadStem(s) => Self::AtlasPathDerivation(s),
            super::AtlasDeriveError::Parse(e) => Self::Path(e),
        }
    }
}

impl AssetLoader for SpineSkeletonJsonLoader {
    type Asset = SpineSkeletonAsset;
    type Settings = SpineSkeletonJsonLoaderSettings;
    type Error = SpineSkeletonJsonLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let atlas_path = resolve_atlas_path(load_context.path(), settings.atlas_path.as_deref())?;

        let atlas_handle: Handle<SpineAtlasAsset> = load_context.load(atlas_path.clone());

        let loaded_atlas = load_context
            .load_builder()
            .load_value::<SpineAtlasAsset>(atlas_path.clone())
            .await
            .map_err(|e| {
                SpineSkeletonJsonLoaderError::AtlasLoad(atlas_path.to_string(), e.to_string())
            })?;
        let atlas_asset = loaded_atlas.get();
        let mut attachment_loader = AtlasAttachmentLoader::new(&atlas_asset.atlas);

        let mut json = SkeletonJson::with_loader(&mut attachment_loader);
        if let Some(scale) = settings.scale {
            json = json.with_scale(scale);
        }
        let data = json.read_slice(&bytes)?;

        Ok(SpineSkeletonAsset {
            data: Arc::new(data),
            atlas: atlas_handle,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

fn resolve_atlas_path(
    json_path: &AssetPath<'static>,
    override_path: Option<&str>,
) -> Result<AssetPath<'static>, SpineSkeletonJsonLoaderError> {
    super::derive_atlas_path(json_path, override_path).map_err(Into::into)
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
        let j = make_path("rigs/spineboy/export/spineboy-pro.json");
        let atlas = resolve_atlas_path(&j, None).unwrap();
        assert_eq!(
            atlas.path().to_str(),
            Some("rigs/spineboy/export/spineboy.atlas")
        );
    }

    #[test]
    fn honours_override() {
        let j = make_path("rigs/spineboy-pro.json");
        let atlas = resolve_atlas_path(&j, Some("packs/hero.atlas")).unwrap();
        assert_eq!(atlas.path().to_str(), Some("packs/hero.atlas"));
    }
}
