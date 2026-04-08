use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, Allocator};

use super::MAX_FRAMES_IN_FLIGHT;

#[allow(dead_code)]
pub enum DeferredDestroy {
    Buffer(vk::Buffer, Allocation),
    ImageAndView(vk::Image, vk::ImageView, Allocation),
    Sampler(vk::Sampler),
    Pipeline(vk::Pipeline),
    PipelineLayout(vk::PipelineLayout),
    DescriptorSetLayout(vk::DescriptorSetLayout),
}

pub struct DestructionQueue {
    queues: [Vec<DeferredDestroy>; MAX_FRAMES_IN_FLIGHT],
    current: usize,
}

impl DestructionQueue {
    pub fn new() -> Self {
        Self {
            queues: [const { Vec::new() }; MAX_FRAMES_IN_FLIGHT],
            current: 0,
        }
    }

    #[allow(dead_code)]
    pub fn push(&mut self, item: DeferredDestroy) {
        self.queues[self.current].push(item);
    }

    pub fn rotate(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        self.current = (self.current + 1) % MAX_FRAMES_IN_FLIGHT;
        let pending = std::mem::take(&mut self.queues[self.current]);
        if !pending.is_empty() {
            let mut alloc = allocator.lock().unwrap();
            destroy_items(device, &mut alloc, pending);
        }
    }

    pub fn flush_all(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        let mut alloc = allocator.lock().unwrap();
        for queue in &mut self.queues {
            let pending = std::mem::take(queue);
            destroy_items(device, &mut alloc, pending);
        }
    }
}

fn destroy_items(device: &ash::Device, alloc: &mut Allocator, items: Vec<DeferredDestroy>) {
    for item in items {
        match item {
            DeferredDestroy::Buffer(buf, allocation) => {
                unsafe { device.destroy_buffer(buf, None) };
                alloc.free(allocation).ok();
            }
            DeferredDestroy::ImageAndView(image, view, allocation) => {
                unsafe {
                    device.destroy_image_view(view, None);
                    device.destroy_image(image, None);
                }
                alloc.free(allocation).ok();
            }
            DeferredDestroy::Sampler(sampler) => unsafe {
                device.destroy_sampler(sampler, None);
            },
            DeferredDestroy::Pipeline(pipeline) => unsafe {
                device.destroy_pipeline(pipeline, None);
            },
            DeferredDestroy::PipelineLayout(layout) => unsafe {
                device.destroy_pipeline_layout(layout, None);
            },
            DeferredDestroy::DescriptorSetLayout(layout) => unsafe {
                device.destroy_descriptor_set_layout(layout, None);
            },
        }
    }
}
