use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ash::vk;
use azalea_core::position::ChunkPos;
use gpu_allocator::vulkan::{Allocation, Allocator};

use super::mesher::ChunkMeshData;
use crate::renderer::util;

const INITIAL_VERTEX_CAPACITY: u64 = 256 * 1024 * 1024;
const INITIAL_INDEX_CAPACITY: u64 = 64 * 1024 * 1024;
const STAGING_CAPACITY: u64 = 32 * 1024 * 1024;
const VERTEX_STRIDE: u64 = std::mem::size_of::<super::mesher::ChunkVertex>() as u64;
const INDEX_STRIDE: u64 = 4;
pub const MAX_CHUNKS: usize = 8192;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndexedIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkAABB {
    pub min: [f32; 4],
    pub max: [f32; 4],
}

struct ChunkSlot {
    vertex_offset: u64,
    vertex_size: u64,
    index_offset: u64,
    index_size: u64,
    index_count: u32,
    aabb: ChunkAABB,
}

struct FreeBlock {
    offset: u64,
    size: u64,
}

struct GpuMegaBuffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    capacity: u64,
    usage: vk::BufferUsageFlags,
    name: &'static str,
    free_list: Vec<FreeBlock>,
}

impl GpuMegaBuffer {
    fn new(
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        capacity: u64,
        usage: vk::BufferUsageFlags,
        name: &'static str,
    ) -> Self {
        let full_usage =
            usage | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;
        let (buffer, allocation) =
            util::create_gpu_buffer(device, allocator, capacity, full_usage, name);
        Self {
            buffer,
            allocation,
            capacity,
            usage,
            name,
            free_list: vec![FreeBlock {
                offset: 0,
                size: capacity,
            }],
        }
    }

    fn alloc_or_grow(
        &mut self,
        size: u64,
        align: u64,
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> Option<u64> {
        if let Some(offset) = self.alloc(size, align) {
            return Some(offset);
        }

        let new_capacity = (self.capacity * 2).max(self.capacity + size);
        log::info!(
            "Growing {} from {}MB to {}MB",
            self.name,
            self.capacity / (1024 * 1024),
            new_capacity / (1024 * 1024)
        );

        unsafe {
            let _ = device.device_wait_idle();
        }

        let full_usage =
            self.usage | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;
        let (new_buffer, new_alloc) =
            util::create_gpu_buffer(device, allocator, new_capacity, full_usage, self.name);

        submit_copies(
            device,
            queue,
            command_pool,
            &[(self.buffer, 0, new_buffer, 0, self.capacity)],
        );

        unsafe { device.destroy_buffer(self.buffer, None) };
        let old_alloc = std::mem::replace(&mut self.allocation, new_alloc);
        allocator.lock().unwrap().free(old_alloc).ok();

        self.buffer = new_buffer;
        self.free_list.push(FreeBlock {
            offset: self.capacity,
            size: new_capacity - self.capacity,
        });
        self.capacity = new_capacity;

        self.alloc(size, align)
    }

    fn alloc(&mut self, size: u64, align: u64) -> Option<u64> {
        for i in 0..self.free_list.len() {
            let block_offset = self.free_list[i].offset;
            let block_size = self.free_list[i].size;
            let aligned = (block_offset + align - 1) & !(align - 1);
            let padding = aligned - block_offset;
            if block_size < size + padding {
                continue;
            }

            let result = aligned;
            let remaining = block_size - size - padding;

            if remaining > 0 {
                self.free_list[i] = FreeBlock {
                    offset: result + size,
                    size: remaining,
                };
            } else {
                self.free_list.remove(i);
            }

            if padding > 0 {
                let pos = self.free_list.partition_point(|b| b.offset < block_offset);
                self.free_list.insert(
                    pos,
                    FreeBlock {
                        offset: block_offset,
                        size: padding,
                    },
                );
            }

            return Some(result);
        }
        None
    }

    fn free(&mut self, offset: u64, size: u64) {
        let pos = self.free_list.partition_point(|b| b.offset < offset);
        self.free_list.insert(pos, FreeBlock { offset, size });

        if pos + 1 < self.free_list.len()
            && self.free_list[pos].offset + self.free_list[pos].size
                == self.free_list[pos + 1].offset
        {
            self.free_list[pos].size += self.free_list[pos + 1].size;
            self.free_list.remove(pos + 1);
        }

        if pos > 0
            && self.free_list[pos - 1].offset + self.free_list[pos - 1].size
                == self.free_list[pos].offset
        {
            self.free_list[pos - 1].size += self.free_list[pos].size;
            self.free_list.remove(pos);
        }
    }

    fn reset(&mut self) {
        self.free_list.clear();
        self.free_list.push(FreeBlock {
            offset: 0,
            size: self.capacity,
        });
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        unsafe { device.destroy_buffer(self.buffer, None) };
        let alloc = std::mem::replace(&mut self.allocation, unsafe { std::mem::zeroed() });
        allocator.lock().unwrap().free(alloc).ok();
    }
}

struct StagingRing {
    buffer: vk::Buffer,
    allocation: Allocation,
    capacity: u64,
    offset: u64,
}

impl StagingRing {
    fn new(device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) -> Self {
        let (buffer, allocation) = util::create_host_buffer(
            device,
            allocator,
            STAGING_CAPACITY,
            vk::BufferUsageFlags::TRANSFER_SRC,
            "staging_ring",
        );
        Self {
            buffer,
            allocation,
            capacity: STAGING_CAPACITY,
            offset: 0,
        }
    }

    fn write(&mut self, data: &[u8]) -> u64 {
        let offset = self.offset;
        let slice = self.allocation.mapped_slice_mut().unwrap();
        slice[offset as usize..offset as usize + data.len()].copy_from_slice(data);
        self.offset += data.len() as u64;
        offset
    }

    fn reset(&mut self) {
        self.offset = 0;
    }

    fn remaining(&self) -> u64 {
        self.capacity - self.offset
    }

    fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        unsafe { device.destroy_buffer(self.buffer, None) };
        let alloc = std::mem::replace(&mut self.allocation, unsafe { std::mem::zeroed() });
        allocator.lock().unwrap().free(alloc).ok();
    }
}

struct PendingCopy {
    src_offset: u64,
    dst_buffer: vk::Buffer,
    dst_offset: u64,
    size: u64,
}

pub struct ChunkBufferStore {
    vertex_mega: GpuMegaBuffer,
    index_mega: GpuMegaBuffer,
    staging: StagingRing,
    pending: Vec<PendingCopy>,
    slots: HashMap<ChunkPos, ChunkSlot>,
}

impl ChunkBufferStore {
    pub fn new(device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) -> Self {
        Self {
            vertex_mega: GpuMegaBuffer::new(
                device,
                allocator,
                INITIAL_VERTEX_CAPACITY,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                "vertex_mega",
            ),
            index_mega: GpuMegaBuffer::new(
                device,
                allocator,
                INITIAL_INDEX_CAPACITY,
                vk::BufferUsageFlags::INDEX_BUFFER,
                "index_mega",
            ),
            staging: StagingRing::new(device, allocator),
            pending: Vec::new(),
            slots: HashMap::new(),
        }
    }

    pub fn upload(
        &mut self,
        mesh: &ChunkMeshData,
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            self.remove(&mesh.pos);
            return;
        }

        self.remove(&mesh.pos);

        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);
        let total_staging = vertex_bytes.len() as u64 + index_bytes.len() as u64;

        if total_staging > self.staging.remaining() {
            self.flush(device, queue, command_pool);
        }

        let Some(vertex_offset) = self.vertex_mega.alloc_or_grow(
            vertex_bytes.len() as u64,
            VERTEX_STRIDE,
            device,
            allocator,
            queue,
            command_pool,
        ) else {
            log::warn!(
                "Vertex buffer allocation failed for chunk [{}, {}]",
                mesh.pos.x,
                mesh.pos.z
            );
            return;
        };
        let Some(index_offset) = self.index_mega.alloc_or_grow(
            index_bytes.len() as u64,
            INDEX_STRIDE,
            device,
            allocator,
            queue,
            command_pool,
        ) else {
            self.vertex_mega
                .free(vertex_offset, vertex_bytes.len() as u64);
            log::warn!(
                "Index buffer allocation failed for chunk [{}, {}]",
                mesh.pos.x,
                mesh.pos.z
            );
            return;
        };

        let v_staging_offset = self.staging.write(vertex_bytes);
        let i_staging_offset = self.staging.write(index_bytes);

        self.pending.push(PendingCopy {
            src_offset: v_staging_offset,
            dst_buffer: self.vertex_mega.buffer,
            dst_offset: vertex_offset,
            size: vertex_bytes.len() as u64,
        });
        self.pending.push(PendingCopy {
            src_offset: i_staging_offset,
            dst_buffer: self.index_mega.buffer,
            dst_offset: index_offset,
            size: index_bytes.len() as u64,
        });

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for v in &mesh.vertices {
            min_y = min_y.min(v.position[1]);
            max_y = max_y.max(v.position[1]);
        }

        let cx = mesh.pos.x as f32 * 16.0;
        let cz = mesh.pos.z as f32 * 16.0;

        self.slots.insert(
            mesh.pos,
            ChunkSlot {
                vertex_offset,
                vertex_size: vertex_bytes.len() as u64,
                index_offset,
                index_size: index_bytes.len() as u64,
                index_count: mesh.indices.len() as u32,
                aabb: ChunkAABB {
                    min: [cx, min_y, cz, 0.0],
                    max: [cx + 16.0, max_y, cz + 16.0, 0.0],
                },
            },
        );
    }

    pub fn flush(
        &mut self,
        device: &ash::Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) {
        if self.pending.is_empty() {
            return;
        }

        let copies: Vec<_> = self
            .pending
            .iter()
            .map(|c| {
                (
                    self.staging.buffer,
                    c.src_offset,
                    c.dst_buffer,
                    c.dst_offset,
                    c.size,
                )
            })
            .collect();

        submit_copies(device, queue, command_pool, &copies);

        self.pending.clear();
        self.staging.reset();
    }

    pub fn remove(&mut self, pos: &ChunkPos) {
        if let Some(slot) = self.slots.remove(pos) {
            self.vertex_mega.free(slot.vertex_offset, slot.vertex_size);
            self.index_mega.free(slot.index_offset, slot.index_size);
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.vertex_mega.reset();
        self.index_mega.reset();
    }

    pub fn vertex_buffer(&self) -> vk::Buffer {
        self.vertex_mega.buffer
    }

    pub fn index_buffer(&self) -> vk::Buffer {
        self.index_mega.buffer
    }

    pub fn chunk_count(&self) -> u32 {
        self.slots.len().min(MAX_CHUNKS) as u32
    }

    pub fn write_draw_data(
        &self,
        commands: &mut [DrawIndexedIndirectCommand],
        aabbs: &mut [ChunkAABB],
    ) -> u32 {
        let count = self.slots.len().min(MAX_CHUNKS);
        for (i, slot) in self.slots.values().take(count).enumerate() {
            commands[i] = DrawIndexedIndirectCommand {
                index_count: slot.index_count,
                instance_count: 1,
                first_index: (slot.index_offset / INDEX_STRIDE) as u32,
                vertex_offset: (slot.vertex_offset / VERTEX_STRIDE) as i32,
                first_instance: 0,
            };
            aabbs[i] = slot.aabb;
        }
        count as u32
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        self.slots.clear();
        self.pending.clear();
        self.vertex_mega.destroy(device, allocator);
        self.index_mega.destroy(device, allocator);
        self.staging.destroy(device, allocator);
    }
}

fn submit_copies(
    device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    copies: &[(vk::Buffer, u64, vk::Buffer, u64, u64)],
) {
    if copies.is_empty() {
        return;
    }

    unsafe {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd = device
            .allocate_command_buffers(&alloc_info)
            .expect("failed to allocate copy command buffer")[0];

        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        device
            .begin_command_buffer(cmd, &begin)
            .expect("failed to begin copy command buffer");

        for &(src, src_offset, dst, dst_offset, size) in copies {
            device.cmd_copy_buffer(
                cmd,
                src,
                dst,
                &[vk::BufferCopy {
                    src_offset,
                    dst_offset,
                    size,
                }],
            );
        }

        device
            .end_command_buffer(cmd)
            .expect("failed to end copy command buffer");

        let cmd_bufs = [cmd];
        let submit = vk::SubmitInfo::default().command_buffers(&cmd_bufs);
        device
            .queue_submit(queue, &[submit], vk::Fence::null())
            .expect("failed to submit copies");
        device
            .queue_wait_idle(queue)
            .expect("failed to wait for copies");
        device.free_command_buffers(command_pool, &[cmd]);
    }
}
