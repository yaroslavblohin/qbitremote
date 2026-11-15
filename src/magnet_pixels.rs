// Analytic U-shaped magnet, supersampled for small Explorer icons.
pub fn rgba(size: u32) -> Vec<u8> {
    let mut pixels = vec![0; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0u32; 4];
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f32 + (sx as f32 + 0.5) / 4.0) / size as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / 4.0) / size as f32;
                    let radius = ((px - 0.5).powi(2) + (py - 0.53).powi(2)).sqrt();
                    let inside = if py >= 0.53 {
                        (0.18..=0.38).contains(&radius)
                    } else {
                        (0.12..=0.88).contains(&px) && !(0.32..0.68).contains(&px) && py >= 0.12
                    };
                    if inside {
                        let c = if py < 0.28 {
                            [224, 232, 240]
                        } else if px < 0.5 {
                            [226, 61, 76]
                        } else {
                            [40, 137, 226]
                        };
                        for i in 0..3 {
                            sum[i] += c[i];
                        }
                        sum[3] += 255;
                    }
                }
            }
            let i = ((y * size + x) * 4) as usize;
            if sum[3] > 0 {
                for c in 0..3 {
                    pixels[i + c] = (sum[c] * 255).checked_div(sum[3]).unwrap_or(0) as u8;
                }
            }
            pixels[i + 3] = (sum[3] / 16) as u8;
        }
    }
    pixels
}
