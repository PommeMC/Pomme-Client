use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

#[derive(Clone)]
pub struct AtlasUVMap {
    regions: HashMap<String, AtlasRegion>,
    missing: AtlasRegion,
}

impl AtlasUVMap {
    pub fn empty() -> Self {
        let missing = AtlasRegion {
            u_min: 0.0,
            v_min: 0.0,
            u_max: 1.0,
            v_max: 1.0,
        };
        Self {
            regions: HashMap::new(),
            missing,
        }
    }

    pub fn get_region(&self, name: &str) -> AtlasRegion {
        self.regions.get(name).copied().unwrap_or(self.missing)
    }
}

pub fn tile_origin(slot: u32, grid_size: u32, tile_size: u32) -> (u32, u32) {
    (
        (slot % grid_size) * tile_size,
        (slot / grid_size) * tile_size,
    )
}

pub fn tile_region(origin: (u32, u32), tile_size: u32, atlas_size: u32) -> AtlasRegion {
    let s = atlas_size as f32;
    AtlasRegion {
        u_min: origin.0 as f32 / s,
        v_min: origin.1 as f32 / s,
        u_max: (origin.0 + tile_size) as f32 / s,
        v_max: (origin.1 + tile_size) as f32 / s,
    }
}
