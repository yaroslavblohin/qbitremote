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
pub fn rasterize(size: u32, extent: f64) -> Vec<u8> {
    let samples = if size <= 64 { 8 } else { 2 };
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    let colors = [
        [0.0, 100.0, 240.0],
        [0.0, 185.0, 230.0],
        [34.0, 197.0, 94.0],
    ];
    for py in 0..size {
        for px in 0..size {
            let mut sum = [0.0; 4];
            for sy in 0..samples {
                for sx in 0..samples {
                    let x = 156.0
                        + ((px as f64 + (sx as f64 + 0.5) / samples as f64) / size as f64 - 0.5)
                            * extent;
                    let y = 90.0
                        + ((py as f64 + (sy as f64 + 0.5) / samples as f64) / size as f64 - 0.5)
                            * extent;
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

// Leave less empty space around the mark while keeping room for its shadow.
const SYSTEM_ICON_EXTENT: f64 = 324.0;

/// Windows shell, window and tray artwork, padded for the cast shadow.
pub fn system_rgba(size: u32) -> Vec<u8> {
    let face = rasterize(size, SYSTEM_ICON_EXTENT);
    let mut result = face.clone();
    let depth = (size as f32 * 0.025).max(0.65);
    let blur = depth * 0.55;
    let alpha = |x: i32, y: i32| {
        if (0..size as i32).contains(&x) && (0..size as i32).contains(&y) {
            face[((y as u32 * size + x as u32) * 4 + 3) as usize] as f32 / 255.0
        } else {
            0.0
        }
    };
    let sample = |x: f32, y: f32| {
        let left = x.floor() as i32;
        let top = y.floor() as i32;
        let fx = x - left as f32;
        let fy = y - top as f32;
        let upper = alpha(left, top) * (1.0 - fx) + alpha(left + 1, top) * fx;
        let lower = alpha(left, top + 1) * (1.0 - fx) + alpha(left + 1, top + 1) * fx;
        upper * (1.0 - fy) + lower * fy
    };
    for (index, pixel) in result.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let opacity = pixel[3] as f32 / 255.0;
        if opacity == 1.0 {
            continue;
        }
        let x = (index % size as usize) as f32 - depth * 0.75;
        let y = (index / size as usize) as f32 - depth * 1.25;
        let shadow = 0.40 * sample(x, y)
            + 0.08
                * (sample(x - blur, y)
                    + sample(x + blur, y)
                    + sample(x, y - blur)
                    + sample(x, y + blur));
        // Composite a black shadow under the face while keeping straight alpha for ICO and tray.
        let combined = opacity + shadow * (1.0 - opacity);
        if combined > 0.0 {
            for channel in &mut pixel[..3] {
                *channel = (*channel as f32 * opacity / combined).round() as u8;
            }
            pixel[3] = (combined * 255.0).round() as u8;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn three_letters_remain_separate_and_background_is_transparent() {
        for size in [16, 32, 64, 256] {
            let pixels = super::rasterize(size, 320.0);
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

    #[test]
    fn system_icons_keep_green_faces_and_visible_shadows_at_small_sizes() {
        for size in [16, 24, 32, 256] {
            let icon = super::system_rgba(size);
            let face = super::rasterize(size, super::SYSTEM_ICON_EXTENT);
            assert_eq!(icon[3], 0);
            // A stroke can run between pixel centers at 16 px.
            let (pixels, _) = icon.as_chunks::<4>();
            let (original_pixels, _) = face.as_chunks::<4>();
            assert!(pixels
                .iter()
                .any(|p| p[1] >= 155 && p[1] > p[2] && p[2] > p[0] && p[3] >= 200));
            assert!(pixels
                .iter()
                .zip(original_pixels)
                .any(|(p, original)| original[3] == 0 && p[3] >= 32));
        }
    }
}
