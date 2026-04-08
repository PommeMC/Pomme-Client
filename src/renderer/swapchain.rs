use std::sync::{Arc, Mutex};

use ash::khr::{surface, swapchain};
use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};

use super::context::ContextError;
use super::util;

pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

#[allow(dead_code)]
pub struct SwapchainState {
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub format: vk::SurfaceFormatKHR,
    pub extent: vk::Extent2D,
    pub depth_image: vk::Image,
    pub depth_view: vk::ImageView,
    pub depth_allocation: Option<Allocation>,
}

impl SwapchainState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &ash::Device,
        surface_loader: &surface::Instance,
        swapchain_loader: &swapchain::Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        width: u32,
        height: u32,
        graphics_family: u32,
        present_family: u32,
        allocator: &Arc<Mutex<Allocator>>,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<Self, ContextError> {
        let caps = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
        };
        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?
        };
        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
        };

        let format = formats
            .iter()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .copied()
            .unwrap_or(formats[0]);

        let present_mode = if present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else if present_modes.contains(&vk::PresentModeKHR::IMMEDIATE) {
            vk::PresentModeKHR::IMMEDIATE
        } else {
            vk::PresentModeKHR::FIFO
        };

        let extent = vk::Extent2D {
            width: width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        };

        let image_count = (caps.min_image_count + 1).min(if caps.max_image_count == 0 {
            u32::MAX
        } else {
            caps.max_image_count
        });

        let (sharing_mode, queue_families) = if graphics_family != present_family {
            (
                vk::SharingMode::CONCURRENT,
                vec![graphics_family, present_family],
            )
        } else {
            (vk::SharingMode::EXCLUSIVE, vec![])
        };

        let swapchain_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(&queue_families)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_info, None)? };
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        let image_views: Vec<vk::ImageView> = images
            .iter()
            .map(|&img| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .subresource_range(util::COLOR_SUBRESOURCE_RANGE);
                unsafe { device.create_image_view(&view_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let (depth_image, depth_view, depth_allocation) =
            create_depth_resources(device, extent, allocator)?;

        Ok(Self {
            swapchain,
            images,
            image_views,
            format,
            extent,
            depth_image,
            depth_view,
            depth_allocation: Some(depth_allocation),
        })
    }

    pub fn depth_format(&self) -> vk::Format {
        DEPTH_FORMAT
    }

    pub fn destroy(
        &mut self,
        device: &ash::Device,
        swapchain_loader: &swapchain::Device,
        allocator: &Arc<Mutex<Allocator>>,
    ) {
        unsafe {
            let _ = device.device_wait_idle();
        }

        unsafe { device.destroy_image_view(self.depth_view, None) };
        if let Some(alloc) = self.depth_allocation.take() {
            allocator.lock().unwrap().free(alloc).ok();
        }
        unsafe { device.destroy_image(self.depth_image, None) };

        for &view in &self.image_views {
            unsafe { device.destroy_image_view(view, None) };
        }
        self.image_views.clear();

        unsafe { swapchain_loader.destroy_swapchain(self.swapchain, None) };
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.extent.width as f32 / self.extent.height.max(1) as f32
    }
}

fn create_depth_resources(
    device: &ash::Device,
    extent: vk::Extent2D,
    allocator: &Arc<Mutex<Allocator>>,
) -> Result<(vk::Image, vk::ImageView, Allocation), ContextError> {
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(DEPTH_FORMAT)
        .extent(vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);

    let image = unsafe { device.create_image(&image_info, None)? };
    let mem_reqs = unsafe { device.get_image_memory_requirements(image) };

    let allocation = allocator.lock().unwrap().allocate(&AllocationCreateDesc {
        name: "depth_image",
        requirements: mem_reqs,
        location: MemoryLocation::GpuOnly,
        linear: false,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
    })?;

    unsafe { device.bind_image_memory(image, allocation.memory(), allocation.offset())? };

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(DEPTH_FORMAT)
        .subresource_range(util::DEPTH_SUBRESOURCE_RANGE);
    let view = unsafe { device.create_image_view(&view_info, None)? };

    Ok((image, view, allocation))
}
