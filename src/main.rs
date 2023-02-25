#![deny(clippy::all)]
#![forbid(unsafe_code)]

use colstodian::spaces::{AcesCg, EncodedSrgb};
use colstodian::tonemap::{PerceptualTonemapper, PerceptualTonemapperParams, Tonemapper};
use colstodian::{Color, Display};
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
    RENDER_BUFFER_HEIGHT, RENDER_BUFFER_SIZE, RENDER_BUFFER_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::gui::Framework;
use crate::image::render_bg_image;

/// Representation of the application state
struct ApplicationState {
    // RGB 32 bit
    framebuffer: [f32; RENDER_BUFFER_SIZE],
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

    let mut app = ApplicationState::new();

    let render_buffer_pointer = Box::new(app.framebuffer);
    println!(
        "Render buffer occupies {} bytes on the stack",
        std::mem::size_of_val(&render_buffer_pointer)
    );

    let (mut pixels, mut framework) = {
        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;

        let surface_texture =
            SurfaceTexture::new(RENDER_BUFFER_WIDTH, RENDER_BUFFER_HEIGHT, &window);

        let pixels = PixelsBuilder::new(RENDER_BUFFER_WIDTH, RENDER_BUFFER_WIDTH, surface_texture)
            .texture_format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .build()?;

        let framework = Framework::new(
            &event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            &pixels,
            render_buffer_pointer,
        );

        (pixels, framework)
    };

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
            &app.update();
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
        let black: f32 = 0.0;
        let mut render_buffer: [f32; RENDER_BUFFER_SIZE] = [black; RENDER_BUFFER_SIZE];
        eprintln!("Size of render buffer: {}", render_buffer.len());
        render_bg_image(&mut render_buffer);

        Self {
            framebuffer: render_buffer,
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
        for (_, (pixel, render_pixel)) in it.enumerate() {
            // Here we draw the pixels!
            // In my case, I already drew them, so I can copy them around
            // and the bits of math to convert from scene referred to display referred

            // Recreate the Scene Linear color struct that we know we used
            // For the sake of simplicity and saving memory, our array is composed of f32
            // instead of propert color structs. Here we recreate the colstodian color struct
            // on the fly so we can do the conversion to 8bit sRGB
            let rendered_color =
                colstodian::color::acescg(render_pixel[0], render_pixel[1], render_pixel[2]);
            let alpha = render_pixel[3];

            // Use a standard Tonemap to go from ACEScg HDR to SDR
            let params = PerceptualTonemapperParams::default();
            let tonemapped: Color<AcesCg, Display> =
                PerceptualTonemapper::tonemap(rendered_color, params).convert();

            // Encode in sRGB so we're ready to display or write to an image
            let encoded = tonemapped.convert::<EncodedSrgb>();

            // Convert to 8bit
            let rgb: [u8; 3] = encoded.to_u8();

            // Can I avoid doing a copy here ?
            let rgba: [u8; 4] = [rgb[0], rgb[1], rgb[2], (255 as f32 * alpha) as u8];

            pixel.copy_from_slice(&rgba);
        }
    }
}
