use std::collections::HashMap;
use std::path::Path;

use azalea_block::BlockState;
use serde::{Deserialize, Serialize};

pub const BLOCK_CACHE_FILE: &str = "block_cache.json";

use crate::assets::AssetIndex;

use super::model::{self, BakedModel};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tint {
    None,
    Grass,
    Foliage,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FaceTextures {
    pub top: String,
    pub bottom: String,
    pub north: String,
    pub south: String,
    pub east: String,
    pub west: String,
    pub side_overlay: Option<String>,
    pub tint: Tint,
}

impl FaceTextures {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        top: &str,
        bottom: &str,
        north: &str,
        south: &str,
        east: &str,
        west: &str,
        side_overlay: Option<&str>,
        tint: Tint,
    ) -> Self {
        Self {
            top: top.into(),
            bottom: bottom.into(),
            north: north.into(),
            south: south.into(),
            east: east.into(),
            west: west.into(),
            side_overlay: side_overlay.map(Into::into),
            tint,
        }
    }

    pub fn uniform(name: &str, tint: Tint) -> Self {
        Self::new(name, name, name, name, name, name, None, tint)
    }
}

#[derive(Clone)]
pub struct BlockRegistry {
    textures: HashMap<String, FaceTextures>,
    baked: HashMap<String, HashMap<String, BakedModel>>,
    multipart: HashMap<String, Vec<model::MultipartEntry>>,
}

impl BlockRegistry {
    pub fn load(
        jar_assets_dir: &Path,
        asset_index: &Option<AssetIndex>,
        game_dir: &Path,
        packs: Option<&crate::resource_pack::ResourcePackManager>,
    ) -> Self {
        let cache_path = game_dir.join(BLOCK_CACHE_FILE);

        let textures = if packs.is_none() {
            if let Some(cached) = load_cache(&cache_path) {
                tracing::info!("Block registry: {} blocks (cached textures)", cached.len());
                Some(cached)
            } else {
                None
            }
        } else {
            None
        };

        let textures = textures.unwrap_or_else(|| {
            let mut textures = model::load_all_block_textures(jar_assets_dir, asset_index, packs);

            textures
                .entry("water".into())
                .or_insert_with(|| FaceTextures::uniform("water_still", Tint::None));
            textures
                .entry("lava".into())
                .or_insert_with(|| FaceTextures::uniform("lava_still", Tint::None));

            save_cache(&cache_path, &textures);
            tracing::info!(
                "Block registry: {} blocks (built and cached)",
                textures.len()
            );
            textures
        });

        let (baked, multipart) = model::bake_all_models(jar_assets_dir, asset_index, packs);

        Self {
            textures,
            baked,
            multipart,
        }
    }

    pub fn get_textures(&self, state: BlockState) -> Option<&FaceTextures> {
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        self.textures.get(block.id())
    }

    pub fn get_baked_model_by_name(&self, name: &str) -> Option<&BakedModel> {
        let variants = self.baked.get(name)?;
        if variants.len() == 1 {
            return variants.values().next();
        }
        variants.get("").or_else(|| variants.values().next())
    }

    pub fn get_baked_model(&self, state: BlockState) -> Option<&BakedModel> {
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        let variants = self.baked.get(block.id())?;

        if variants.len() == 1 {
            return variants.values().next();
        }

        let variant_key = build_variant_key(&*block);
        variants
            .get(&variant_key)
            .or_else(|| variants.get(""))
            .or_else(|| variants.values().next())
    }

    pub fn get_multipart_quads(&self, state: BlockState) -> Option<Vec<&model::BakedQuad>> {
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        let entries = self.multipart.get(block.id())?;
        let props = block.property_map();

        let mut quads = Vec::new();
        for entry in entries {
            if entry.when.is_empty()
                || entry
                    .when
                    .iter()
                    .all(|(k, v)| props.get(k.as_str()).map(|pv| pv == v).unwrap_or(false))
            {
                quads.extend(entry.quads.iter());
            }
        }

        if quads.is_empty() { None } else { Some(quads) }
    }

    /// Returns the visual AABB of a block in local [0,1] space, matching vanilla's
    /// `getVisualShape`. Returns `None` for air and transparent blocks (glass, etc.)
    /// that the camera should pass through.
    pub fn visual_shape(&self, state: BlockState) -> Option<crate::physics::aabb::Aabb> {
        use crate::physics::aabb::Aabb;
        let full = || Some(Aabb::new(glam::Vec3::ZERO, glam::Vec3::ONE));

        if state.is_air() {
            return None;
        }
        let block: Box<dyn azalea_block::BlockTrait> = state.into();
        if is_transparent_block(block.id()) {
            return None;
        }
        if let Some(model) = self.get_baked_model(state) {
            if model.is_full_cube {
                return full();
            }
            return aabb_from_quads(model.quads.iter()).or_else(full);
        }
        if let Some(quads) = self.get_multipart_quads(state) {
            return aabb_from_quads(quads.into_iter()).or_else(full);
        }
        full()
    }

    pub fn is_opaque_full_cube(&self, state: BlockState) -> bool {
        if state.is_air() {
            return false;
        }
        self.get_baked_model(state)
            .map(|m| m.is_full_cube)
            .unwrap_or(false)
    }

    pub fn texture_names(&self) -> impl Iterator<Item = &str> + '_ {
        let face_textures = self.textures.values().flat_map(|ft| {
            let base = [
                &ft.top, &ft.bottom, &ft.north, &ft.south, &ft.east, &ft.west,
            ];
            base.into_iter()
                .map(|s| s.as_str())
                .chain(ft.side_overlay.as_deref())
        });

        let baked_textures = self.baked.values().flat_map(|variants| {
            variants
                .values()
                .flat_map(|model| model.quads.iter().map(|q| q.texture.as_str()))
        });

        let multipart_textures = self.multipart.values().flat_map(|entries| {
            entries
                .iter()
                .flat_map(|e| e.quads.iter().map(|q| q.texture.as_str()))
        });

        face_textures
            .chain(baked_textures)
            .chain(multipart_textures)
    }
}

fn build_variant_key(block: &dyn azalea_block::BlockTrait) -> String {
    let props = block.property_map();
    if props.is_empty() {
        return String::new();
    }
    let mut entries: Vec<_> = props.iter().collect();
    entries.sort_by_key(|(k, _)| *k);
    entries
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Blocks where vanilla's `getVisualShape` returns empty (TransparentBlock subclasses).
/// Camera passes through these.
fn is_transparent_block(id: &str) -> bool {
    matches!(
        id,
        "glass"
            | "white_stained_glass"
            | "orange_stained_glass"
            | "magenta_stained_glass"
            | "light_blue_stained_glass"
            | "yellow_stained_glass"
            | "lime_stained_glass"
            | "pink_stained_glass"
            | "gray_stained_glass"
            | "light_gray_stained_glass"
            | "cyan_stained_glass"
            | "purple_stained_glass"
            | "blue_stained_glass"
            | "brown_stained_glass"
            | "green_stained_glass"
            | "red_stained_glass"
            | "black_stained_glass"
            | "tinted_glass"
            | "copper_grate"
            | "exposed_copper_grate"
            | "weathered_copper_grate"
            | "oxidized_copper_grate"
            | "waxed_copper_grate"
            | "waxed_exposed_copper_grate"
            | "waxed_weathered_copper_grate"
            | "waxed_oxidized_copper_grate"
    )
}

fn aabb_from_quads<'a>(quads: impl Iterator<Item = &'a super::model::BakedQuad>) -> Option<crate::physics::aabb::Aabb> {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    let mut any = false;
    for q in quads {
        for pos in &q.positions {
            for i in 0..3 {
                // Baked quad positions are already in [0..1] block space
                min[i] = min[i].min(pos[i]);
                max[i] = max[i].max(pos[i]);
            }
            any = true;
        }
    }
    if !any {
        return None;
    }
    Some(crate::physics::aabb::Aabb::new(
        glam::Vec3::new(min[0], min[1], min[2]),
        glam::Vec3::new(max[0], max[1], max[2]),
    ))
}

fn load_cache(path: &Path) -> Option<HashMap<String, FaceTextures>> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_cache(path: &Path, textures: &HashMap<String, FaceTextures>) {
    if let Ok(json) = serde_json::to_string(textures)
        && let Err(e) = std::fs::write(path, json)
    {
        tracing::warn!("Failed to write block cache: {e}");
    }
}
