//! Small, scalable transfer icon shared by the window and notification area.
pub fn rgba(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0_f32; 4];
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f32 + (sx as f32 + 0.5) / 4.0) / size as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / 4.0) / size as f32;
                    let dx = (px - 0.5).abs() - 0.26;
                    let dy = (py - 0.5).abs() - 0.26;
                    if dx.max(0.0).hypot(dy.max(0.0)) > 0.20 {
                        continue;
                    }
                    let down = [
                        (0.33, 0.25, 0.33, 0.72),
                        (0.19, 0.58, 0.33, 0.72),
                        (0.47, 0.58, 0.33, 0.72),
                    ];
                    let up = [
                        (0.67, 0.75, 0.67, 0.28),
                        (0.53, 0.42, 0.67, 0.28),
                        (0.81, 0.42, 0.67, 0.28),
                    ];
                    let on_line = |&(ax, ay, bx, by): &(f32, f32, f32, f32)| {
                        let t = (((px - ax) * (bx - ax) + (py - ay) * (by - ay))
                            / ((bx - ax).powi(2) + (by - ay).powi(2)))
                        .clamp(0.0, 1.0);
                        (px - ax - t * (bx - ax)).hypot(py - ay - t * (by - ay)) <= 0.045
                    };
                    let color = if down.iter().any(on_line) {
                        [15.0, 47.0, 59.0]
                    } else if up.iter().any(on_line) {
                        [240.0, 255.0, 254.0]
                    } else {
                        [62.0 - py * 16.0, 207.0 - py * 31.0, 195.0 - py * 13.0]
                    };
                    for i in 0..3 {
                        sum[i] += color[i];
                    }
                    sum[3] += 255.0;
                }
            }
            let alpha = sum[3] / 16.0;
            for channel in &sum[..3] {
                pixels.push(if sum[3] > 0.0 {
                    (channel * 255.0 / sum[3]).round() as u8
                } else {
                    0
                });
            }
            pixels.push(alpha.round() as u8);
        }
    }
    pixels
}

pub fn window() -> eframe::egui::IconData {
    eframe::egui::IconData {
        rgba: rgba(64),
        width: 64,
        height: 64,
    }
}
