//! Common transparent raster master for GUI, tray and Windows icon resources.
const SIDE: usize = 256;
const MASTER: &[u8; SIDE * SIDE * 4] = include_bytes!("../assets/logo.rgba");

/// Area-sampled bilinear resizing in premultiplied alpha avoids dark fringes and
/// keeps the transfer arrows readable in the 16px notification-area resource.
pub fn rgba(size: u32) -> Vec<u8> {
    if size == 0 {
        return Vec::new();
    }
    let samples = (SIDE as u32 / size).clamp(1, 8);
    let mut output = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0_f32; 4];
            for sy in 0..samples {
                for sx in 0..samples {
                    let fx = ((x as f32 + (sx as f32 + 0.5) / samples as f32) * SIDE as f32
                        / size as f32
                        - 0.5)
                        .clamp(0.0, (SIDE - 1) as f32);
                    let fy = ((y as f32 + (sy as f32 + 0.5) / samples as f32) * SIDE as f32
                        / size as f32
                        - 0.5)
                        .clamp(0.0, (SIDE - 1) as f32);
                    let x0 = fx.floor() as usize;
                    let y0 = fy.floor() as usize;
                    let tx = fx - x0 as f32;
                    let ty = fy - y0 as f32;
                    for (px, py, weight) in [
                        (x0, y0, (1.0 - tx) * (1.0 - ty)),
                        ((x0 + 1).min(SIDE - 1), y0, tx * (1.0 - ty)),
                        (x0, (y0 + 1).min(SIDE - 1), (1.0 - tx) * ty),
                        ((x0 + 1).min(SIDE - 1), (y0 + 1).min(SIDE - 1), tx * ty),
                    ] {
                        let index = (py * SIDE + px) * 4;
                        let alpha = MASTER[index + 3] as f32 * weight;
                        for c in 0..3 {
                            sum[c] += MASTER[index + c] as f32 * alpha;
                        }
                        sum[3] += alpha;
                    }
                }
            }
            for c in &sum[..3] {
                output.push(if sum[3] == 0.0 {
                    0
                } else {
                    (c / sum[3]).round() as u8
                });
            }
            output.push((sum[3] / (samples * samples) as f32).round() as u8);
        }
    }
    output
}
