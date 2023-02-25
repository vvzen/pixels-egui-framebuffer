use colstodian::spaces::EncodedSrgb;
use colstodian::{color, Color, Display, Oklab};

use crate::constants::{FRAMEBUFFER_HEIGHT, FRAMEBUFFER_SIZE, FRAMEBUFFER_WIDTH};

/// Linear remap a value in one range into another range (no clamping)
pub fn fit_range(x: f32, imin: f32, imax: f32, omin: f32, omax: f32) -> f32 {
    (omax - omin) * (x - imin) / (imax - imin) + omin
}

pub fn render_bg_image(pixels: &mut [u8; FRAMEBUFFER_SIZE]) {
    let mut index: usize = 0;
    for x in 0..FRAMEBUFFER_WIDTH {
        for y in 0..FRAMEBUFFER_HEIGHT {
            // Get normalized U,V coordinates as we move through the image
            let u = fit_range(x as f32, 0.0, FRAMEBUFFER_WIDTH as f32, 0.0, 1.0);
            let v = fit_range(y as f32, 0.0, FRAMEBUFFER_HEIGHT as f32, 0.0, 1.0);

            // Generate a gradient between two colors in LAB space
            let red = color::srgb_u8(255, 0, 0).convert::<Oklab>();
            let blue = color::srgb_u8(0, 0, 255).convert::<Oklab>();
            let green = color::srgb_u8(0, 0, 255).convert::<Oklab>();
            let h_blended = red.blend(green, u);
            let v_blended = h_blended.blend(blue, v);

            // Convert to display referred
            let output: Color<EncodedSrgb, Display> = v_blended.convert();

            // Can I avoid doing a copy here ?
            let rgb: [u8; 3] = output.to_u8();

            // R, G, B, A
            pixels[index + 0] = rgb[0];
            pixels[index + 1] = rgb[1];
            pixels[index + 2] = rgb[2];
            pixels[index + 3] = 0xff;
            index += 4;
        }
    }
}
