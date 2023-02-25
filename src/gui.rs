use egui::{ClippedPrimitive, Context, TexturesDelta};
use egui_wgpu::renderer::{Renderer, ScreenDescriptor};
use pixels::{wgpu, PixelsContext};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

/// Manages all state required for rendering egui over `Pixels`.
pub(crate) struct Framework {
    // State for egui.
    egui_ctx: Context,
    egui_state: egui_winit::State,
    screen_descriptor: ScreenDescriptor,
    renderer: Renderer,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,

    // State for the GUI
    gui: Gui,
}

/// Example application state. A real application will need a lot more state than this.
struct Gui {
    scale_factor: f32,
    // UI options
    window_open: bool,
    should_rerender: bool,
    window_width: u32,
    window_height: u32,
    file_path: String,
    color_a: [u8; 4],
    color_b: [u8; 4],
    file_format_chosen: FileFormat,
}

#[derive(Debug, PartialEq)]
enum FileFormat {
    OpenEXR,
}

impl Framework {
    /// Create egui.
    pub(crate) fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        width: u32,
        height: u32,
        scale_factor: f32,
        pixels: &pixels::Pixels,
    ) -> Self {
        let max_texture_size = pixels.device().limits().max_texture_dimension_2d as usize;

        let egui_ctx = Context::default();
        let mut egui_state = egui_winit::State::new(event_loop);
        egui_state.set_max_texture_side(max_texture_size);
        egui_state.set_pixels_per_point(scale_factor);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };
        let renderer = Renderer::new(pixels.device(), pixels.render_texture_format(), None, 1);
        let textures = TexturesDelta::default();
        let gui = Gui::new(width, height, scale_factor);

        Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            renderer,
            paint_jobs: Vec::new(),
            textures,
            gui,
        }
    }

    /// Handle input events from the window manager.
    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        let _ = self.egui_state.on_event(&self.egui_ctx, event);
    }

    /// Resize egui.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.screen_descriptor.size_in_pixels = [width, height];
        }
    }

    /// Update scaling factor.
    pub(crate) fn scale_factor(&mut self, scale_factor: f64) {
        self.screen_descriptor.pixels_per_point = scale_factor as f32;
    }

    /// Prepare egui.
    pub(crate) fn prepare(&mut self, window: &Window) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx);
        });

        self.textures.append(output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(output.shapes);
    }

    /// Render egui.
    pub(crate) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) {
        // Upload all resources to the GPU.
        for (id, image_delta) in &self.textures.set {
            self.renderer
                .update_texture(&context.device, &context.queue, *id, image_delta);
        }
        self.renderer.update_buffers(
            &context.device,
            &context.queue,
            encoder,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        // Render egui with WGPU
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.renderer
                .render(&mut rpass, &self.paint_jobs, &self.screen_descriptor);
        };
        // Cleanup
        let textures = std::mem::take(&mut self.textures);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
    }
}

impl Gui {
    /// Create a `Gui`.
    fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            window_open: true,
            should_rerender: false,
            window_width: width,
            window_height: height,
            file_path: String::new(),
            color_a: [0x00, 0x00, 0x00, 0xff],
            color_b: [0xff, 0xff, 0xff, 0xff],
            scale_factor,
            file_format_chosen: FileFormat::OpenEXR,
        }
    }

    /// Create the UI using egui.
    fn ui(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("About...").clicked() {
                        self.window_open = true;
                        ui.close_menu();
                    }
                })
            });
        });

        egui::Window::new("Scene Options")
            .open(&mut self.window_open)
            .default_pos(egui::Pos2::new(
                self.window_width as f32 * (1.0 / self.scale_factor) * 0.015,
                self.window_height as f32 * (1.0 / self.scale_factor) * 0.10,
            ))
            .show(ctx, |ui| {
                egui::Grid::new("grid_1").show(ui, |ui| {
                    ui.label("First Color:");
                    ui.color_edit_button_srgba_unmultiplied(&mut self.color_a);
                    ui.label("Second Color:");
                    ui.color_edit_button_srgba_unmultiplied(&mut self.color_b);
                    ui.end_row();
                });

                ui.separator();

                if ui.button("Render").clicked() {
                    self.should_rerender = true;
                    eprintln!("Re-rendering...");
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.0;
                    ui.label("Learn more about egui at");
                    ui.hyperlink("https://docs.rs/egui");
                });
            });

        egui::Window::new("Save Options")
            .open(&mut self.window_open)
            .default_pos(egui::Pos2::new(
                self.window_width as f32 * (1.0 / self.scale_factor) * 0.75,
                self.window_height as f32 * (1.0 / self.scale_factor) * 0.10,
            ))
            .show(ctx, |ui| {
                egui::ComboBox::from_label("Output format")
                    .selected_text(format!("{:?}", self.file_format_chosen))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.file_format_chosen,
                            FileFormat::OpenEXR,
                            "OpenEXR",
                        );
                    });

                ui.separator();
                ui.spacing_mut().item_spacing.x /= 2.0;

                ui.label("File name:");
                ui.text_edit_singleline(&mut self.file_path);
                if ui.button("Save").clicked() {
                    // Here goes your save logic
                }
            });
    }
}
