use colstodian::spaces::{AcesCg, EncodedSrgb};
use colstodian::{color, Color, Display, Oklab, Scene};
use exr::prelude::{AnyChannel, AnyChannels, FlatSamples};
use smallvec::smallvec;

use crate::constants::{RENDER_BUFFER_HEIGHT, RENDER_BUFFER_SIZE, RENDER_BUFFER_WIDTH};

/// Linear remap a value in one range into another range (no clamping)
pub fn fit_range(x: f32, imin: f32, imax: f32, omin: f32, omax: f32) -> f32 {
    (omax - omin) * (x - imin) / (imax - imin) + omin
}

pub fn render_bg_image(render_buffer: &mut [f32; RENDER_BUFFER_SIZE]) {
    let mut index: usize = 0;
    for x in 0..RENDER_BUFFER_WIDTH {
        for y in 0..RENDER_BUFFER_HEIGHT {
            // Get normalized U,V coordinates as we move through the image
            let u = fit_range(x as f32, 0.0, RENDER_BUFFER_WIDTH as f32, 0.0, 1.0);
            let v = fit_range(y as f32, 0.0, RENDER_BUFFER_HEIGHT as f32, 0.0, 1.0);

            // Generate a gradient between two colors in AcesCG
            // TODO: Could we do this in LAB, and then convert to ACES CG ?
            let red = color::srgb_u8(255, 0, 0).convert::<AcesCg>();
            let blue = color::srgb_u8(0, 0, 255).convert::<AcesCg>();
            let green = color::srgb_u8(0, 255, 0).convert::<AcesCg>();
            let h_blended = red.blend(green, u);
            let v_blended = red.blend(blue, v);
            let final_color = h_blended.blend(v_blended, 0.5);

            // Let's just pretend this is fine..
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

pub fn pixels_array_to_exr_channels(
    render_buffer: &mut [f32; RENDER_BUFFER_SIZE],
) -> AnyChannels<FlatSamples> {
    // A vec for each channel
    let mut r_vec: Vec<f32> = Vec::new();
    let mut g_vec: Vec<f32> = Vec::new();
    let mut b_vec: Vec<f32> = Vec::new();

    for f32_color in render_buffer.chunks_exact(3) {
        r_vec.push(f32_color[0]);
        g_vec.push(f32_color[1]);
        b_vec.push(f32_color[2]);
    }

    // Save the data into the channels
    let r_channel = AnyChannel::new("R", FlatSamples::F32(r_vec));
    let g_channel = AnyChannel::new("G", FlatSamples::F32(g_vec));
    let b_channel = AnyChannel::new("B", FlatSamples::F32(b_vec));

    let channels = AnyChannels::sort(smallvec![r_channel, g_channel, b_channel]);
    channels
}
