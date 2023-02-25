#![deny(clippy::all)]
#![forbid(unsafe_code)]

use log::error;
use pixels::{wgpu, Error, PixelsBuilder, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod constants;
mod gui;
mod image;

use crate::constants::{
    FRAMEBUFFER_HEIGHT, FRAMEBUFFER_SIZE, FRAMEBUFFER_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::gui::Framework;
use crate::image::render_bg_image;

/// Representation of the application state
struct ApplicationState {
    // RGBA 8 bit
    framebuffer: [u8; FRAMEBUFFER_SIZE],
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Sample Framebuffer in Pixels + egui")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;

        // let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let surface_texture = SurfaceTexture::new(FRAMEBUFFER_WIDTH, FRAMEBUFFER_HEIGHT, &window);

        let pixels = PixelsBuilder::new(FRAMEBUFFER_WIDTH, FRAMEBUFFER_WIDTH, surface_texture)
            .texture_format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .build()?;

        let framework = Framework::new(
            &event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            &pixels,
        );

        (pixels, framework)
    };
    let mut app = ApplicationState::new();

    event_loop.run(move |event, _, control_flow| {
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Update the scale factor
            if let Some(scale_factor) = input.scale_factor() {
                framework.scale_factor(scale_factor);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    error!("pixels.resize_surface() failed: {err}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                framework.resize(size.width, size.height);
            }

            // Update internal state and request a redraw
            app.update();
            window.request_redraw();
        }

        match event {
            Event::WindowEvent { event, .. } => {
                // Update egui inputs
                framework.handle_event(&event);
            }
            // Draw the current frame
            Event::RedrawRequested(_) => {
                // Draw the world
                app.draw(pixels.get_frame_mut());

                // Prepare egui
                framework.prepare(&window);

                // Render everything together
                // TODO: I really don't want the texture to alway scale
                // up to the whole window, how can I achieve that?
                let render_result = pixels.render_with(|encoder, render_target, context| {
                    // Render the world texture
                    context.scaling_renderer.render(encoder, render_target);

                    // Render egui
                    framework.render(encoder, render_target, context);

                    Ok(())
                });

                // Basic error handling
                if let Err(err) = render_result {
                    error!("pixels.render() failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => (),
        }
    });
}

impl ApplicationState {
    /// Create a new `ApplicationState` instance that can draw a moving box.
    fn new() -> Self {
        // Start from black
        let mut pixels: [u8; FRAMEBUFFER_SIZE] = [0x00; FRAMEBUFFER_SIZE];
        render_bg_image(&mut pixels);

        Self {
            framebuffer: pixels,
        }
    }

    /// Update the Application internal state
    fn update(&mut self) {
        // TODO: here goes any update logic
    }

    // Draw to the frame buffer
    // Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    // This means:
    //     Red, green, blue, and alpha channels.
    //     8 bit integer per channel.
    //     Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader
    // See more formats here: https://docs.rs/wgpu/latest/wgpu/enum.TextureFormat.html
    fn draw(&self, frame: &mut [u8]) {
        let it = std::iter::zip(frame.chunks_exact_mut(4), self.framebuffer.chunks_exact(4));
        for (_, (pixel, other_pixel)) in it.enumerate() {
            // Here we draw the pixels!
            // In my case, I already drew them, so I can copy them around
            pixel.copy_from_slice(other_pixel);
        }
    }
}
