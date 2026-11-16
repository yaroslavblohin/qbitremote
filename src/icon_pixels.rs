//! Approved rounded qbr monogram. Coordinates also define assets/brand-monogram.svg.
fn capsule(x: f64, y: f64, ax: f64, ay: f64, bx: f64, by: f64, radius: f64) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (x - ax - t * dx).powi(2) + (y - ay - t * dy).powi(2) <= radius * radius
}
fn q(x: f64, y: f64) -> bool {
    let radius = (x - 58.0).hypot(y - 90.0);
    (22.0..=48.0).contains(&radius) || capsule(x, y, 93.0, 90.0, 93.0, 151.0, 13.0)
}
fn letter(x: f64, y: f64) -> Option<usize> {
    if q(x, y) {
        return Some(0);
    }
    if q(224.0 - x, 180.0 - y) {
        return Some(1);
    }
    if !(226.0..=302.0).contains(&x) || !(48.0..=138.0).contains(&y) {
        return None;
    }
    if capsule(x, y, 239.0, 90.0, 239.0, 125.0, 13.0) {
        return Some(2);
    }
    // Broad rounded r shoulder, sampled from a cubic Bezier centerline.
    let mut previous = (239.0, 90.0);
    for step in 1..=24 {
        let t = step as f64 / 24.0;
        let u = 1.0 - t;
        let point = (
            u * u * u * 239.0
                + 3.0 * u * u * t * 239.0
                + 3.0 * u * t * t * 258.0
                + t * t * t * 289.0,
            u * u * u * 90.0 + 3.0 * u * u * t * 67.0 + 3.0 * u * t * t * 61.0 + t * t * t * 61.0,
        );
        if capsule(x, y, previous.0, previous.1, point.0, point.1, 13.0) {
            return Some(2);
        }
        previous = point;
    }
    None
}
pub fn rgba(size: u32) -> Vec<u8> {
    let samples = if size <= 64 { 8 } else { 2 };
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    let colors = [
        [0.0, 100.0, 240.0],
        [0.0, 185.0, 230.0],
        [0.0, 184.0, 194.0],
    ];
    for py in 0..size {
        for px in 0..size {
            let mut sum = [0.0; 4];
            for sy in 0..samples {
                for sx in 0..samples {
                    let x = (px as f64 + (sx as f64 + 0.5) / samples as f64) * 320.0 / size as f64
                        - 4.0;
                    let y = (py as f64 + (sy as f64 + 0.5) / samples as f64) * 320.0 / size as f64
                        - 70.0;
                    if let Some(index) = letter(x, y) {
                        for (channel, value) in sum[..3].iter_mut().enumerate() {
                            *value += colors[index][channel];
                        }
                        sum[3] += 1.0;
                    }
                }
            }
            for value in &sum[..3] {
                out.push(if sum[3] > 0.0 {
                    (value / sum[3]) as u8
                } else {
                    0
                });
            }
            out.push((sum[3] * 255.0 / (samples * samples) as f64).round() as u8);
        }
    }
    out
}
#[cfg(test)]
mod tests {
    #[test]
    fn three_letters_remain_separate_and_background_is_transparent() {
        for size in [16, 32, 64, 256] {
            let pixels = super::rgba(size);
            assert_eq!(pixels.len(), (size * size * 4) as usize);
            assert_eq!(pixels[3], 0);
            for x in [110.0, 220.0] {
                let px = ((x + 4.0) / 320.0 * size as f64).floor() as usize;
                if size >= 64 {
                    assert!(
                        (0..size as usize).all(|y| pixels[(y * size as usize + px) * 4 + 3] < 255)
                    );
                }
            }
            for index in 0..3 {
                assert!((10..310).any(|x| super::letter(x as f64, 90.0) == Some(index)));
            }
        }
    }
}
