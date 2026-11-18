/// Draws the NetPulse tray/application icon as RGBA pixels.
pub fn rgba(size: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let radius = size as f32 / 2.0 - 1.0;
    let center = size as f32 / 2.0;
    let scale = size as f32 / 32.0;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let dx = fx - center;
            let dy = fy - center;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius {
                continue;
            }
            // Dark disc so the icon reads on light and dark taskbars.
            let edge = ((radius - distance) * 2.0).clamp(0.0, 1.0);
            let mut color = [30u8, 41, 59, (edge * 255.0) as u8];

            let sx = fx / scale;
            let sy = fy / scale;

            // Down arrow (green), left half.
            let shaft = (9.0..12.0).contains(&sx) && (7.0..17.0).contains(&sy);
            let head = (17.0..23.0).contains(&sy) && (sx - 10.5).abs() <= (23.0 - sy) * 0.65;
            if shaft || head {
                color = [34, 197, 94, 255];
            }

            // Up arrow (blue), right half.
            let shaft = (20.0..23.0).contains(&sx) && (15.0..25.0).contains(&sy);
            let head = (9.0..15.0).contains(&sy) && (sx - 21.5).abs() <= (sy - 9.0) * 0.65;
            if shaft || head {
                color = [56, 189, 248, 255];
            }

            let index = ((y * size + x) * 4) as usize;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
    pixels
}
