use std::sync::Arc;

use bevy::asset::{AssetLoader, AssetPath, LoadContext, io::Reader};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use spine_runtime::load::{AtlasAttachmentLoader, JsonError, SkeletonJson};

use crate::asset::atlas_loader::SpineAtlasAsset;
use crate::asset::skel_loader::SpineSkeletonAsset;

/// Per-load overrides for the JSON skeleton loader. When `atlas_path` is
/// `None` (the default), the loader derives the atlas path from the
/// skeleton's filename stem — the same convention used by the binary
/// `.skel` loader (`spineboy-pro.json` -> `spineboy.atlas`).
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct SpineSkeletonJsonLoaderSettings {
    /// Absolute asset path of the atlas, or `None` to auto-derive.
    pub atlas_path: Option<String>,
    /// Uniform scale applied to vertex coordinates at load time. `None` keeps
    /// the skeleton's native scale. Forwarded to `SkeletonJson::with_scale`.
    pub scale: Option<f32>,
}

/// Bevy asset loader for `.json` skeleton files. Loads the companion atlas
/// the same way the `.skel` loader does and yields a [`SpineSkeletonAsset`]
/// — the asset type is shared so both formats plug into the rest of the
/// pipeline identically.
#[derive(Default, TypePath)]
pub struct SpineSkeletonJsonLoader;

#[derive(Debug, Error)]
pub enum SpineSkeletonJsonLoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not derive atlas path from skeleton path {0:?}")]
    AtlasPathDerivation(String),
    #[error("failed to load companion atlas {0:?}: {1}")]
    AtlasLoad(String, String),
    #[error("json skeleton parse error: {0}")]
    Parse(#[from] JsonError),
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
