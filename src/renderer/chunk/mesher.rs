use std::sync::Arc;

use azalea_core::position::ChunkPos;

use crate::renderer::chunk::atlas::{AtlasRegion, AtlasUVMap};
use crate::world::block::registry::{BlockRegistry, FaceTextures, Tint};
use crate::world::chunk::{self, ChunkStore};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub light: f32,
    pub tint: [f32; 3],
}


pub struct ChunkMeshData {
    pub pos: ChunkPos,
    pub vertices: Vec<ChunkVertex>,
    pub indices: Vec<u32>,
}

struct Face {
    positions: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    offset: [i32; 3],
    light: f32,
}

const FACES: [Face; 6] = [
    // Top (Y+): viewed from above, +X right, -Z up
    Face {
        positions: [
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        offset: [0, 1, 0],
        light: 1.0,
    },
    // Bottom (Y-): viewed from below, +X right, +Z up
    Face {
        positions: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        offset: [0, -1, 0],
        light: 0.5,
    },
    // North (Z-): viewed from -Z, +X right, +Y up
    Face {
        positions: [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        uvs: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
        offset: [0, 0, -1],
        light: 0.7,
    },
    // South (Z+): viewed from +Z, +X left, +Y up
    Face {
        positions: [
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        uvs: [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
        offset: [0, 0, 1],
        light: 0.7,
    },
    // East (X+): viewed from +X, -Z right, +Y up
    Face {
        positions: [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
        uvs: [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
        offset: [1, 0, 0],
        light: 0.8,
    },
    // West (X-): viewed from -X, +Z right, +Y up
    Face {
        positions: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        uvs: [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
        offset: [-1, 0, 0],
        light: 0.8,
    },
];

fn face_texture(textures: &FaceTextures, face_idx: usize) -> &str {
    match face_idx {
        0 => textures.top,
        1 => textures.bottom,
        2 => textures.north,
        3 => textures.south,
        4 => textures.east,
        _ => textures.west,
    }
}

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
// TODO: Replace hardcoded tints with biome colormap sampling (grass.png/foliage.png)
// Plains biome: grass.png at temp=0.8, downfall=0.4 → #91BD59
const GRASS_TINT: [f32; 3] = [0.569, 0.741, 0.349];
// Plains biome: foliage.png at temp=0.8, downfall=0.4 → #77AB2F
const FOLIAGE_TINT: [f32; 3] = [0.467, 0.671, 0.184];

fn tint_color(tint: Tint) -> [f32; 3] {
    match tint {
        Tint::None => WHITE,
        Tint::Grass => GRASS_TINT,
        Tint::Foliage => FOLIAGE_TINT,
    }
}

const MAX_MESH_UPLOADS_PER_FRAME: usize = 4;

pub struct MeshDispatcher {
    result_rx: crossbeam_channel::Receiver<ChunkMeshData>,
    result_tx: crossbeam_channel::Sender<ChunkMeshData>,
    registry: Arc<BlockRegistry>,
    uv_map: Arc<AtlasUVMap>,
}

impl MeshDispatcher {
    pub fn new(registry: BlockRegistry, uv_map: AtlasUVMap) -> Self {
        let (result_tx, result_rx) = crossbeam_channel::unbounded();
        Self {
            result_rx,
            result_tx,
            registry: Arc::new(registry),
            uv_map: Arc::new(uv_map),
        }
    }

    pub fn enqueue(&self, chunk_store: &ChunkStore, pos: ChunkPos) {
        let registry = Arc::clone(&self.registry);
        let uv_map = Arc::clone(&self.uv_map);
        let tx = self.result_tx.clone();

        let chunks_needed = [
            pos,
            ChunkPos::new(pos.x - 1, pos.z),
            ChunkPos::new(pos.x + 1, pos.z),
            ChunkPos::new(pos.x, pos.z - 1),
            ChunkPos::new(pos.x, pos.z + 1),
        ];
        let chunk_arcs: Vec<_> = chunks_needed
            .iter()
            .map(|p| chunk_store.get_chunk(p))
            .collect();

        let min_y = chunk_store.min_y();
        let height = chunk_store.height();

        rayon::spawn(move || {
            let snapshot = ChunkStoreSnapshot {
                chunks: chunks_needed
                    .into_iter()
                    .zip(chunk_arcs)
                    .collect(),
                min_y,
                height,
            };
            let mesh = mesh_chunk_snapshot(&snapshot, pos, &registry, &uv_map);
            let _ = tx.send(mesh);
        });
    }

    pub fn drain_results(&self) -> impl Iterator<Item = ChunkMeshData> + '_ {
        self.result_rx.try_iter().take(MAX_MESH_UPLOADS_PER_FRAME)
    }
}

struct ChunkStoreSnapshot {
    chunks: Vec<(ChunkPos, Option<Arc<parking_lot::RwLock<azalea_world::chunk_storage::Chunk>>>)>,
    min_y: i32,
    height: u32,
}

impl ChunkStoreSnapshot {
    fn get_block_state(&self, x: i32, y: i32, z: i32) -> azalea_block::BlockState {
        let chunk_pos = ChunkPos::new(x.div_euclid(16), z.div_euclid(16));
        let chunk_lock = self
            .chunks
            .iter()
            .find(|(p, _)| *p == chunk_pos)
            .and_then(|(_, c)| c.as_ref());

        let Some(chunk_lock) = chunk_lock else {
            return azalea_block::BlockState::AIR;
        };

        let c = chunk_lock.read();
        chunk::block_state_from_section(&c, x, y, z, self.min_y)
    }

    fn min_y(&self) -> i32 {
        self.min_y
    }

    fn height(&self) -> u32 {
        self.height
    }
}

fn mesh_chunk_snapshot(
    snapshot: &ChunkStoreSnapshot,
    pos: ChunkPos,
    registry: &BlockRegistry,
    uv_map: &AtlasUVMap,
) -> ChunkMeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let min_y = snapshot.min_y();
    let max_y = min_y + snapshot.height() as i32;
    let world_x = pos.x * 16;
    let world_z = pos.z * 16;

    for local_z in 0..16i32 {
        for local_x in 0..16i32 {
            let bx = world_x + local_x;
            let bz = world_z + local_z;

            for by in min_y..max_y {
                let state = snapshot.get_block_state(bx, by, bz);
                if state.is_air() {
                    continue;
                }

                let textures = match registry.get_textures(state) {
                    Some(t) => t,
                    None => continue,
                };

                let block_pos = [bx as f32, by as f32, bz as f32];

                let tint = tint_color(textures.tint);
                let is_side = |idx: usize| idx >= 2;

                for (i, face) in FACES.iter().enumerate() {
                    let neighbor = snapshot.get_block_state(
                        bx + face.offset[0],
                        by + face.offset[1],
                        bz + face.offset[2],
                    );
                    if neighbor.is_air() {
                        let tex_name = face_texture(textures, i);
                        let region = uv_map.get_region(tex_name);

                        if let Some(overlay) = textures.side_overlay.filter(|_| is_side(i)) {
                            emit_face(&mut vertices, &mut indices, block_pos, face, region, WHITE);
                            let overlay_region = uv_map.get_region(overlay);
                            emit_face(&mut vertices, &mut indices, block_pos, face, overlay_region, tint);
                        } else {
                            let is_tinted_face = !matches!(textures.tint, Tint::None)
                                && (textures.side_overlay.is_none() || i == 0);
                            let face_tint = if is_tinted_face { tint } else { WHITE };
                            emit_face(&mut vertices, &mut indices, block_pos, face, region, face_tint);
                        }
                    }
                }
            }
        }
    }

    ChunkMeshData {
        pos,
        vertices,
        indices,
    }
}

fn emit_face(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    block_pos: [f32; 3],
    face: &Face,
    region: AtlasRegion,
    tint: [f32; 3],
) {
    let base = vertices.len() as u32;

    let u_span = region.u_max - region.u_min;
    let v_span = region.v_max - region.v_min;

    for (i, pos) in face.positions.iter().enumerate() {
        let uv = face.uvs[i];
        vertices.push(ChunkVertex {
            position: [
                block_pos[0] + pos[0],
                block_pos[1] + pos[1],
                block_pos[2] + pos[2],
            ],
            tex_coords: [
                region.u_min + uv[0] * u_span,
                region.v_min + uv[1] * v_span,
            ],
            light: face.light,
            tint,
        });
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
}
