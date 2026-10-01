use std::sync::Arc;

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::image::Image;
use bevy::prelude::*;
use thiserror::Error;

use spine_runtime::atlas::{Atlas, AtlasError};

/// A parsed `.atlas` file and an image handle per page.
#[derive(Asset, TypePath, Debug)]
pub struct SpineAtlasAsset {
    pub atlas: Arc<Atlas>,
    /// `pages[i]` is the image for `atlas.pages[i]`; a render command's
    /// `TextureId` is an index into it.
    pub pages: Vec<Handle<Image>>,
}

/// Loads `.atlas` files and each page image, resolved relative to the atlas.
#[derive(Default, TypePath)]
pub struct SpineAtlasLoader;

#[derive(Debug, Error)]
pub enum SpineAtlasLoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("atlas is not valid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("atlas parse error: {0}")]
    Parse(#[from] AtlasError),
}

impl AssetLoader for SpineAtlasLoader {
    type Asset = SpineAtlasAsset;
    type Settings = ();
    type Error = SpineAtlasLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let text = std::str::from_utf8(&bytes)?;
        let atlas = Atlas::parse(text)?;

        let mut pages = Vec::with_capacity(atlas.pages.len());
        let base_path = load_context.path().clone();
        for page in &atlas.pages {
            let png_path = base_path
                .resolve_embed_str(&page.name)
                .unwrap_or_else(|_| base_path.clone());
            let handle: Handle<Image> = load_context.load(png_path);
            pages.push(handle);
        }

        Ok(SpineAtlasAsset {
            atlas: Arc::new(atlas),
            pages,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["atlas"]
    }
}
