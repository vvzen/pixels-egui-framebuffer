use std::path::Path;

use anyhow;
use colstodian::spaces::{AcesCg, EncodedSrgb};
use colstodian::{color, Color, Display, Oklab, Scene};
use exr::prelude::{
    AnyChannel, AnyChannels, Encoding, FlatSamples, Image, Layer, LayerAttributes, WritableImage,
};
use smallvec::smallvec;

use crate::constants::{RENDER_BUFFER_HEIGHT, RENDER_BUFFER_SIZE, RENDER_BUFFER_WIDTH};

/// Linear remap a value in one range into another range (no clamping)
pub fn fit_range(x: f32, imin: f32, imax: f32, omin: f32, omax: f32) -> f32 {
    (omax - omin) * (x - imin) / (imax - imin) + omin
}

pub fn render_bg_image(render_buffer: &mut [f32; RENDER_BUFFER_SIZE]) {
    let mut index: usize = 0;
    for y in (0..RENDER_BUFFER_HEIGHT).rev() {
        for x in 0..RENDER_BUFFER_WIDTH {
            // Get normalized U,V coordinates as we move through the image
            let u = fit_range(x as f32, 0.0, RENDER_BUFFER_WIDTH as f32, 0.0, 1.0);
            let v = fit_range(y as f32, 0.0, RENDER_BUFFER_HEIGHT as f32, 0.0, 1.0);

            // Generate a gradient between two colors in AcesCG
            // TODO: Could we do this in LAB, and then convert to ACES CG ?
            let red = color::acescg::<Scene>(1.0, 0.0, 0.0);
            let blue = color::acescg::<Scene>(0.0, 0.0, 1.0);
            let green = color::acescg::<Scene>(0.0, 1.0, 0.0);
            let h_blended = red.blend(green, u);
            let v_blended = red.blend(blue, v);
            let final_color = h_blended.blend(v_blended, 0.5);

            // Here I was playing around with Color Spaces
            // let red = fit_range(x as f32, 0.0, RENDER_BUFFER_WIDTH as f32, 0.0, 1.0);
            // let green = fit_range(y as f32, 0.0, RENDER_BUFFER_HEIGHT as f32, 0.0, 1.0);
            // let blue = 0.25;

            // let rd = color::acescg::<Display>(red, green, blue);
            // let rendered_color: Color<AcesCg, Scene> = rd.convert_state(|f| f);

            let rendered_color =
                color::acescg::<Scene>(final_color.r, final_color.g, final_color.b);

            // R, G, B, A
            render_buffer[index + 0] = rendered_color.r;
            render_buffer[index + 1] = rendered_color.g;
            render_buffer[index + 2] = rendered_color.b;
            render_buffer[index + 3] = 1.0;

            index += 4;
        }
    }
}

pub fn write_as_exr_image(
    image_path: impl AsRef<Path>,
    width: usize,
    height: usize,
    render_buffer: &Box<[f32; RENDER_BUFFER_SIZE]>,
) -> anyhow::Result<()> {
    let resolution = (width, height);

    // A vec for each channel
    let mut r_vec: Vec<f32> = Vec::new();
    let mut g_vec: Vec<f32> = Vec::new();
    let mut b_vec: Vec<f32> = Vec::new();

    for f32_color in render_buffer.chunks_exact(4) {
        r_vec.push(f32_color[0]);
        g_vec.push(f32_color[1]);
        b_vec.push(f32_color[2]);
    }

    // Save the data into the channels
    let r_channel = AnyChannel::new("R", FlatSamples::F32(r_vec));
    let g_channel = AnyChannel::new("G", FlatSamples::F32(g_vec));
    let b_channel = AnyChannel::new("B", FlatSamples::F32(b_vec));

    let channels = AnyChannels::sort(smallvec![r_channel, g_channel, b_channel]);

    // The layer attributes can store additional metadata
    let mut layer_attributes = LayerAttributes::named("rgb");
    layer_attributes.comments = Some("Generated by vvzen from Rust".into());
    layer_attributes.owner = Some("vvzen".into());
    layer_attributes.software_name = Some("rust-tracer".into());

    // The only layer in this image
    let layer = Layer::new(
        resolution,
        layer_attributes,
        Encoding::SMALL_LOSSLESS,
        channels,
    );

    // Write the image to disk
    let image = Image::from_layer(layer);
    match image.write().to_file(&image_path) {
        Ok(_) => {
            eprintln!(
                "Successfully saved image to {}",
                image_path.as_ref().display()
            );
        }
        Err(e) => {
            anyhow::bail!("Failed to write image: {e:?}");
        }
    }

    Ok(())
}
