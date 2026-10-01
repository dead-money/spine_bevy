use std::sync::Arc;

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::image::Image;
use bevy::prelude::*;
use thiserror::Error;

use spine_runtime::atlas::{Atlas, AtlasError};

/// Parsed Spine `.atlas` file plus a page-index-parallel `Vec<Handle<Image>>`
/// that the Bevy-side renderer uses to resolve `TextureId(page_index)` into a
/// concrete `Handle<Image>`.
#[derive(Asset, TypePath, Debug)]
pub struct SpineAtlasAsset {
    /// Arc-shared parsed atlas. Multiple skeleton assets can reference the
    /// same atlas cheaply.
    pub atlas: Arc<Atlas>,
    /// `pages[i]` is the GPU image for `atlas.pages[i]`. Indexed by
    /// `RenderCommand::texture.0 as usize` at draw time.
    pub pages: Vec<Handle<Image>>,
}

/// Bevy asset loader for `.atlas` files. Parses the atlas text and
/// triggers a dependent `Image` load for every page so all PNGs land in
/// the asset server alongside the atlas itself.
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

        // Resolve each page's PNG as a sibling of the .atlas file and register
        // it as a dependency. `resolve_embed` is RFC-1808 relative resolution
        // (strips the atlas filename, concatenates the PNG name).
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
