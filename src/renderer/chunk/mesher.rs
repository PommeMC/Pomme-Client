use std::sync::Arc;

use azalea_core::position::ChunkPos;

use crate::renderer::chunk::atlas::{AtlasRegion, AtlasUVMap};
use crate::world::block::model::{BakedModel, Direction};
use crate::world::block::registry::{BlockRegistry, Tint};
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

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const GRASS_TINT: [f32; 3] = [0.569, 0.741, 0.349];
const FOLIAGE_TINT: [f32; 3] = [0.467, 0.671, 0.184];

fn tint_color(tint: Tint) -> [f32; 3] {
    match tint {
        Tint::None => WHITE,
        Tint::Grass => GRASS_TINT,
        Tint::Foliage => FOLIAGE_TINT,
    }
}

const MAX_MESH_UPLOADS_PER_FRAME: usize = 256;

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
                chunks: chunks_needed.into_iter().zip(chunk_arcs).collect(),
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
    chunks: Vec<(
        ChunkPos,
        Option<Arc<parking_lot::RwLock<azalea_world::chunk_storage::Chunk>>>,
    )>,
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
    let mut logged_missing: std::collections::HashSet<String> = std::collections::HashSet::new();

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

                let block_pos = [bx as f32, by as f32, bz as f32];

                if let Some(baked) = registry.get_baked_model(state) {
                    emit_baked_model(
                        &mut vertices,
                        &mut indices,
                        block_pos,
                        baked,
                        snapshot,
                        registry,
                        uv_map,
                        bx,
                        by,
                        bz,
                    );
                } else if let Some(textures) = registry.get_textures(state) {
                    emit_cube_faces(
                        &mut vertices,
                        &mut indices,
                        block_pos,
                        textures,
                        snapshot,
                        registry,
                        uv_map,
                        bx,
                        by,
                        bz,
                    );
                } else {
                    let block: Box<dyn azalea_block::BlockTrait> = state.into();
                    let id = block.id().to_string();
                    if logged_missing.insert(id.clone()) {
                        log::warn!("Missing model: {id}");
                    }
                    emit_missing_cube(
                        &mut vertices,
                        &mut indices,
                        block_pos,
                        snapshot,
                        registry,
                        bx,
                        by,
                        bz,
                    );
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

fn emit_baked_model(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    block_pos: [f32; 3],
    model: &BakedModel,
    snapshot: &ChunkStoreSnapshot,
    registry: &BlockRegistry,
    uv_map: &AtlasUVMap,
    bx: i32,
    by: i32,
    bz: i32,
) {
    for quad in &model.quads {
        if let Some(cullface) = quad.cullface {
            let offset = cullface.offset();
            let neighbor = snapshot.get_block_state(bx + offset[0], by + offset[1], bz + offset[2]);
            if registry.is_opaque_full_cube(neighbor) {
                continue;
            }
        }

        let region = uv_map.get_region(&quad.texture);
        let tint = if quad.tinted { GRASS_TINT } else { WHITE };
        emit_face(
            vertices,
            indices,
            block_pos,
            &quad.positions,
            &quad.uvs,
            quad.shade_light,
            region,
            tint,
        );
    }
}

fn emit_cube_faces(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    block_pos: [f32; 3],
    textures: &crate::world::block::registry::FaceTextures,
    snapshot: &ChunkStoreSnapshot,
    registry: &BlockRegistry,
    uv_map: &AtlasUVMap,
    bx: i32,
    by: i32,
    bz: i32,
) {
    let tint = tint_color(textures.tint);

    for (i, dir) in CUBE_FACE_DIRS.iter().enumerate() {
        let offset = dir.offset();
        let neighbor = snapshot.get_block_state(bx + offset[0], by + offset[1], bz + offset[2]);
        if registry.is_opaque_full_cube(neighbor) {
            continue;
        }

        let face_tex = match i {
            0 => &textures.top,
            1 => &textures.bottom,
            2 => &textures.north,
            3 => &textures.south,
            4 => &textures.east,
            _ => &textures.west,
        };
        let region = uv_map.get_region(face_tex);
        let (positions, uvs, light) = cube_face_geometry(*dir);

        let is_side = i >= 2;
        if let Some(overlay) = textures.side_overlay.as_deref().filter(|_| is_side) {
            emit_face(
                vertices, indices, block_pos, &positions, &uvs, light, region, WHITE,
            );
            let overlay_region = uv_map.get_region(overlay);
            emit_face(
                vertices,
                indices,
                block_pos,
                &positions,
                &uvs,
                light,
                overlay_region,
                tint,
            );
        } else {
            let is_tinted =
                !matches!(textures.tint, Tint::None) && (textures.side_overlay.is_none() || i == 0);
            let face_tint = if is_tinted { tint } else { WHITE };
            emit_face(
                vertices, indices, block_pos, &positions, &uvs, light, region, face_tint,
            );
        }
    }
}

const MISSING_TINT: [f32; 3] = [1.0, 0.0, 1.0];

fn emit_missing_cube(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    block_pos: [f32; 3],
    snapshot: &ChunkStoreSnapshot,
    registry: &BlockRegistry,
    bx: i32,
    by: i32,
    bz: i32,
) {
    for dir in &CUBE_FACE_DIRS {
        let offset = dir.offset();
        let neighbor = snapshot.get_block_state(bx + offset[0], by + offset[1], bz + offset[2]);
        if registry.is_opaque_full_cube(neighbor) {
            continue;
        }

        let (positions, _, light) = cube_face_geometry(*dir);
        let base = vertices.len() as u32;
        for pos in &positions {
            vertices.push(ChunkVertex {
                position: [
                    block_pos[0] + pos[0],
                    block_pos[1] + pos[1],
                    block_pos[2] + pos[2],
                ],
                tex_coords: [0.0, 0.0],
                light,
                tint: MISSING_TINT,
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }
}

const CUBE_FACE_DIRS: [Direction; 6] = [
    Direction::Up,
    Direction::Down,
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
];

fn emit_face(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    block_pos: [f32; 3],
    positions: &[[f32; 3]; 4],
    uvs: &[[f32; 2]; 4],
    light: f32,
    region: AtlasRegion,
    tint: [f32; 3],
) {
    let base = vertices.len() as u32;
    let u_span = region.u_max - region.u_min;
    let v_span = region.v_max - region.v_min;

    for i in 0..4 {
        vertices.push(ChunkVertex {
            position: [
                block_pos[0] + positions[i][0],
                block_pos[1] + positions[i][1],
                block_pos[2] + positions[i][2],
            ],
            tex_coords: [
                region.u_min + uvs[i][0] * u_span,
                region.v_min + uvs[i][1] * v_span,
            ],
            light,
            tint,
        });
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
}

fn cube_face_geometry(dir: Direction) -> ([[f32; 3]; 4], [[f32; 2]; 4], f32) {
    match dir {
        Direction::Up => (
            [
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            1.0,
        ),
        Direction::Down => (
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            0.5,
        ),
        Direction::North => (
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            0.7,
        ),
        Direction::South => (
            [
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
            0.7,
        ),
        Direction::East => (
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
            [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
            0.8,
        ),
        Direction::West => (
            [
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
            [[1.0, 1.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]],
            0.8,
        ),
    }
}
