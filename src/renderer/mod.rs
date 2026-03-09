pub mod camera;
pub mod chunk;
mod context;
pub(crate) mod shader;
mod swapchain;

use std::path::Path;
use std::sync::Arc;

use ash::vk;
use azalea_core::position::ChunkPos;
use thiserror::Error;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use camera::Camera;
use chunk::mesher::{ChunkMeshData, MeshDispatcher};
use context::VulkanContext;
use swapchain::SwapchainState;

use crate::assets::AssetIndex;
use crate::window::input::InputState;
use crate::world::block::registry::BlockRegistry;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("failed to initialize GPU context: {0}")]
    Context(#[from] context::ContextError),

    #[error("vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct Renderer {
    ctx: VulkanContext,
    swapchain: SwapchainState,
    camera: Camera,
    registry: BlockRegistry,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    swapchain_dirty: bool,
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(
        window: Arc<Window>,
        _assets_dir: &Path,
        _asset_index: &Option<AssetIndex>,
    ) -> Result<Self, RendererError> {
        let size = window.inner_size();
        let ctx = VulkanContext::new(&window)?;

        let swapchain_state = SwapchainState::new(
            &ctx.device,
            &ctx.surface_loader,
            &ctx.swapchain_loader,
            ctx.physical_device,
            ctx.surface,
            size.width.max(1),
            size.height.max(1),
            ctx.graphics_family,
            ctx.present_family,
            &ctx.allocator,
        )?;

        let camera = Camera::new(swapchain_state.aspect_ratio());
        let registry = BlockRegistry::new();

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            None,
            None,
            None,
        );

        Ok(Self {
            ctx,
            swapchain: swapchain_state,
            camera,
            registry,
            egui_state,
            egui_ctx,
            swapchain_dirty: false,
            width: size.width.max(1),
            height: size.height.max(1),
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.width = new_size.width;
        self.height = new_size.height;
        self.swapchain_dirty = true;
        self.camera
            .set_aspect_ratio(new_size.width as f32 / new_size.height as f32);
    }

    fn recreate_swapchain(&mut self) -> Result<(), RendererError> {
        self.swapchain.destroy(
            &self.ctx.device,
            &self.ctx.swapchain_loader,
            &self.ctx.allocator,
        );
        self.swapchain = SwapchainState::new(
            &self.ctx.device,
            &self.ctx.surface_loader,
            &self.ctx.swapchain_loader,
            self.ctx.physical_device,
            self.ctx.surface,
            self.width,
            self.height,
            self.ctx.graphics_family,
            self.ctx.present_family,
            &self.ctx.allocator,
        )?;
        self.swapchain_dirty = false;
        Ok(())
    }

    pub fn handle_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.egui_state.on_window_event(window, event)
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    pub fn update_camera(&mut self, input: &mut InputState) {
        self.camera.update_look(input);
    }

    pub fn sync_camera_to_player(&mut self, eye_pos: glam::Vec3, yaw: f32, pitch: f32) {
        self.camera.position = eye_pos;
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;
    }

    pub fn camera_yaw(&self) -> f32 {
        self.camera.yaw
    }

    pub fn camera_pitch(&self) -> f32 {
        self.camera.pitch
    }

    pub fn set_camera_position(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.camera
            .set_position(glam::Vec3::new(x as f32, y as f32, z as f32), yaw, pitch);
    }

    pub fn upload_chunk_mesh(&mut self, _mesh: &ChunkMeshData) {
        // TODO: Phase 8 step 3 — Vulkan chunk mesh upload
    }

    pub fn remove_chunk_mesh(&mut self, _pos: &ChunkPos) {
        // TODO: Phase 8 step 3
    }

    pub fn clear_chunk_meshes(&mut self) {
        // TODO: Phase 8 step 3
    }

    pub fn create_mesh_dispatcher(&self) -> MeshDispatcher {
        MeshDispatcher::new(self.registry.clone(), chunk::atlas::AtlasUVMap::empty())
    }

    pub fn render_world(
        &mut self,
        window: &Window,
        hide_cursor: bool,
        _hud_fn: impl FnMut(&egui::Context),
    ) -> Result<(), RendererError> {
        self.render_frame(window, hide_cursor, [0.529, 0.808, 0.922, 1.0])
    }

    pub fn render_ui(
        &mut self,
        window: &Window,
        _scroll: f32,
        _ui_fn: impl FnMut(&egui::Context),
    ) -> Result<(), RendererError> {
        self.render_frame(window, false, [0.0, 0.0, 0.0, 1.0])
    }

    fn render_frame(
        &mut self,
        _window: &Window,
        _hide_cursor: bool,
        clear_color: [f32; 4],
    ) -> Result<(), RendererError> {
        if self.swapchain_dirty {
            self.recreate_swapchain()?;
        }

        let frame = self.ctx.frame_index;
        let fence = self.ctx.in_flight_fences[frame];
        let image_available = self.ctx.image_available[frame];
        let render_finished = self.ctx.render_finished[frame];
        let cmd = self.ctx.command_buffers[frame];

        unsafe {
            self.ctx
                .device
                .wait_for_fences(&[fence], true, u64::MAX)?;
        }

        let image_index = match unsafe {
            self.ctx.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                image_available,
                vk::Fence::null(),
            )
        } {
            Ok((idx, _)) => idx,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.swapchain_dirty = true;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        unsafe {
            self.ctx.device.reset_fences(&[fence])?;
            self.ctx.device.reset_command_buffer(
                cmd,
                vk::CommandBufferResetFlags::empty(),
            )?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.ctx.device.begin_command_buffer(cmd, &begin_info)?;

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: clear_color,
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.swapchain.render_pass)
                .framebuffer(self.swapchain.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(&clear_values);

            self.ctx.device.cmd_begin_render_pass(
                cmd,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            // TODO: Draw chunks, egui, etc.

            self.ctx.device.cmd_end_render_pass(cmd);
            self.ctx.device.end_command_buffer(cmd)?;

            let wait_semaphores = [image_available];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [render_finished];
            let cmd_buffers = [cmd];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&cmd_buffers)
                .signal_semaphores(&signal_semaphores);

            self.ctx
                .device
                .queue_submit(self.ctx.graphics_queue, &[submit_info], fence)?;

            let swapchains = [self.swapchain.swapchain];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            match self
                .ctx
                .swapchain_loader
                .queue_present(self.ctx.present_queue, &present_info)
            {
                Ok(false) => {}
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.swapchain_dirty = true;
                }
                Err(e) => return Err(e.into()),
            }
        }

        self.ctx.advance_frame();
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { let _ = self.ctx.device.device_wait_idle(); }
        self.swapchain.destroy(
            &self.ctx.device,
            &self.ctx.swapchain_loader,
            &self.ctx.allocator,
        );
    }
}
