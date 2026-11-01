//! qBit Remote monogram with a transfer remote; shared by GUI, tray and EXE.
fn capsule(x: f32, y: f32, a: (f32, f32), b: (f32, f32), radius: f32) -> bool {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let t = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (x - a.0 - t * dx).hypot(y - a.1 - t * dy) <= radius
}
fn rounded(x: f32, y: f32, cx: f32, cy: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let dx = (x - cx).abs() - hx + r;
    let dy = (y - cy).abs() - hy + r;
    dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r
}
fn sample(x: f32, y: f32, size: u32) -> Option<[f32; 3]> {
    let edge = rounded(x, y, 0.49, 0.49, 0.45, 0.45, 0.22);
    let mut color = if edge <= 0.0 {
        let t = (0.36 * x + 0.64 * y).clamp(0.0, 1.0);
        Some(if edge > -0.012 {
            [73.0 - 28.0 * t, 178.0 - 73.0 * t, 255.0 - 30.0 * t]
        } else {
            [35.0 - 8.0 * t, 151.0 - 88.0 * t, 240.0 - 63.0 * t]
        })
    } else {
        None
    };
    let cyan = [107.0, 222.0, 255.0];
    let q = (x - 0.30).hypot(y - 0.46);
    let b = (x - 0.64).hypot(y - 0.46);
    if (q - 0.12).abs() <= 0.033 || capsule(x, y, (0.415, 0.46), (0.415, 0.70), 0.033) {
        color = Some(cyan);
    }
    if (b - 0.12).abs() <= 0.033 || capsule(x, y, (0.525, 0.215), (0.525, 0.46), 0.033) {
        color = Some([246.0, 251.0, 255.0]);
    }
    // A tilted TV-style remote: power key, download/upload rocker and small keys.
    // Separate from the monogram, with no cable or server-rack badge.
    let dx = x - 0.79;
    let dy = y - 0.77;
    let u = dx * 0.9487 + dy * 0.3162;
    let v = -dx * 0.3162 + dy * 0.9487;
    let remote = rounded(u, v, 0.0, 0.0, 0.112, 0.20, 0.055);
    if remote <= 0.0 {
        color = Some(if remote > -0.012 {
            [134.0, 219.0, 252.0]
        } else {
            [13.0, 31.0, 54.0]
        });
        // The two transfer arrows deliberately use the same colors as q and b.
        let stroke = if size <= 24 { 0.019 } else { 0.012 };
        for (cx, direction) in [(-0.043, 1.0_f32), (0.043, -1.0)] {
            if rounded(u, v, cx, 0.01, 0.034, 0.083, 0.025) <= 0.0 {
                color = Some([30.0, 62.0, 94.0]);
            }
            if capsule(
                u,
                v,
                (cx, -0.048 * direction),
                (cx, 0.066 * direction),
                stroke,
            ) || capsule(
                u,
                v,
                (cx - 0.023, 0.039 * direction),
                (cx, 0.066 * direction),
                stroke,
            ) || capsule(
                u,
                v,
                (cx + 0.023, 0.039 * direction),
                (cx, 0.066 * direction),
                stroke,
            ) {
                color = Some(if direction > 0.0 {
                    cyan
                } else {
                    [246.0, 251.0, 255.0]
                });
            }
        }
        let power = u.hypot(v + 0.137);
        if size >= 48 {
            if (power - 0.022).abs() < 0.005 && !(v < -0.149 && u.abs() < 0.011)
                || capsule(u, v, (0.0, -0.166), (0.0, -0.142), 0.005)
            {
                color = Some(cyan);
            }
            for cx in [-0.037, 0.037] {
                for cy in [0.125, 0.156] {
                    if (u - cx).hypot(v - cy) <= 0.008 {
                        color = Some([134.0, 176.0, 205.0]);
                    }
                }
            }
        } else if power < 0.015 {
            color = Some(cyan);
        }
    }
    color
}
pub fn rgba(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0_f32; 3];
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f32 + (sx as f32 + 0.5) / 4.0) / size as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / 4.0) / size as f32;
                    if let Some(color) = sample(px, py, size) {
                        for i in 0..3 {
                            sum[i] += color[i];
                        }
                        covered += 1;
                    }
                }
            }
            for channel in sum {
                pixels.push(if covered == 0 {
                    0
                } else {
                    (channel / covered as f32).round() as u8
                });
            }
            pixels.push((covered as f32 * 255.0 / 16.0).round() as u8);
        }
    }
    pixels
}
