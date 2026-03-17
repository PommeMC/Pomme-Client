use std::io::Cursor;
use std::sync::Arc;

use azalea_block::BlockState;
use azalea_core::position::ChunkPos;
use azalea_world::chunk_storage::{Chunk, ChunkStorage, PartialChunkStorage};
use azalea_world::heightmap::HeightmapKind;
use parking_lot::RwLock;
use thiserror::Error;

const OVERWORLD_HEIGHT: u32 = 384;
const OVERWORLD_MIN_Y: i32 = -64;

#[derive(Error, Debug)]
pub enum ChunkError {
    #[error("failed to parse chunk data: {0}")]
    Parse(String),
}

pub struct ChunkStore {
    pub chunk_storage: ChunkStorage,
    pub partial_storage: PartialChunkStorage,
}

impl ChunkStore {
    pub fn new(view_distance: u32) -> Self {
        Self::new_with_dimension(view_distance, OVERWORLD_HEIGHT, OVERWORLD_MIN_Y)
    }

    pub fn new_with_dimension(view_distance: u32, height: u32, min_y: i32) -> Self {
        Self {
            chunk_storage: ChunkStorage::new(height, min_y),
            partial_storage: PartialChunkStorage::new(view_distance.max(64)),
        }
    }

    pub fn load_chunk(
        &mut self,
        pos: ChunkPos,
        data: &[u8],
        heightmaps: &[(HeightmapKind, Box<[u64]>)],
    ) -> Result<(), ChunkError> {
        let mut cursor = Cursor::new(data);
        self.partial_storage
            .replace_with_packet_data(&pos, &mut cursor, heightmaps, &mut self.chunk_storage)
            .map_err(|e| ChunkError::Parse(e.to_string()))
    }

    pub fn unload_chunk(&mut self, pos: &ChunkPos) {
        self.partial_storage.limited_set(pos, None);
    }

    pub fn set_center(&mut self, pos: ChunkPos) {
        self.partial_storage.update_view_center(pos);
    }

    pub fn get_chunk(&self, pos: &ChunkPos) -> Option<Arc<RwLock<Chunk>>> {
        self.partial_storage.limited_get(pos).cloned()
    }

    pub fn set_block_state(&self, x: i32, y: i32, z: i32, state: BlockState) {
        let chunk_pos = ChunkPos::new(x.div_euclid(16), z.div_euclid(16));
        let Some(chunk_lock) = self.get_chunk(&chunk_pos) else {
            return;
        };
        let mut chunk = chunk_lock.write();
        let block_pos = azalea_core::position::ChunkBlockPos {
            x: x.rem_euclid(16) as u8,
            y,
            z: z.rem_euclid(16) as u8,
        };
        chunk.set_block_state(&block_pos, state, self.chunk_storage.min_y);
    }

    pub fn get_block_state(&self, x: i32, y: i32, z: i32) -> BlockState {
        let chunk_pos = ChunkPos::new(x.div_euclid(16), z.div_euclid(16));
        let Some(chunk_lock) = self.get_chunk(&chunk_pos) else {
            return BlockState::AIR;
        };
        let chunk = chunk_lock.read();
        block_state_from_section(&chunk, x, y, z, self.chunk_storage.min_y)
    }

    pub fn height(&self) -> u32 {
        self.chunk_storage.height
    }

    pub fn min_y(&self) -> i32 {
        self.chunk_storage.min_y
    }
}

pub fn block_state_from_section(chunk: &Chunk, x: i32, y: i32, z: i32, min_y: i32) -> BlockState {
    let section_idx = ((y - min_y) / 16) as usize;
    if section_idx >= chunk.sections.len() {
        return BlockState::AIR;
    }

    let local_x = x.rem_euclid(16) as u8;
    let local_y = (y - min_y).rem_euclid(16) as u8;
    let local_z = z.rem_euclid(16) as u8;

    chunk.sections[section_idx].get_block_state(azalea_core::position::ChunkSectionBlockPos {
        x: local_x,
        y: local_y,
        z: local_z,
    })
}
