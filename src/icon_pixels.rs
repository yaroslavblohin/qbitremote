//! Deterministic monogram: b is exactly q rotated 180 degrees, before common shear.
const Q: &[(f64, f64)] = &[
    (32., 96.),
    (40., 88.),
    (104., 88.),
    (112., 96.),
    (112., 192.),
    (124., 192.),
    (99., 216.),
    (74., 192.),
    (86., 192.),
    (86., 168.),
    (40., 168.),
    (32., 160.),
];
fn q(x: f64, y: f64) -> bool {
    // Ray-cast outer outline; the same rectangular counter is shared by both letters.
    let mut hit = false;
    for i in 0..Q.len() {
        let (ax, ay) = Q[i];
        let (bx, by) = Q[(i + 1) % Q.len()];
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            hit = !hit;
        }
    }
    hit && !((60.0..84.0).contains(&x) && (116.0..140.0).contains(&y))
}
fn mask(x: f64, y: f64) -> bool {
    q(x - 16.0, y) || q(240.0 - x, 256.0 - y)
}
fn separator(x: f64, y: f64) -> bool {
    (x - 128.0).abs() < 1.7 && (88.0..=168.0).contains(&y)
}
fn color(y: f64) -> [f64; 3] {
    let stops = [
        (0., [236., 245., 255.]),
        (96., [122., 143., 165.]),
        (108., [249., 253., 255.]),
        (114., [35., 226., 238.]),
        (136., [3., 166., 217.]),
        (158., [104., 47., 236.]),
        (171., [228., 126., 255.]),
        (177., [138., 19., 220.]),
        (256., [73., 8., 152.]),
    ];
    for pair in stops.windows(2) {
        if y <= pair[1].0 {
            let t = ((y - pair[0].0) / (pair[1].0 - pair[0].0)).clamp(0., 1.);
            return std::array::from_fn(|i| pair[0].1[i] * (1. - t) + pair[1].1[i] * t);
        }
    }
    stops[stops.len() - 1].1
}
pub fn rgba(size: u32) -> Vec<u8> {
    let samples = if size <= 64 { 4 } else { 2 };
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let mut sum = [0.; 4];
            for sy in 0..samples {
                for sx in 0..samples {
                    let x = (px as f64 + (sx as f64 + 0.5) / samples as f64) * 256. / size as f64;
                    let y = (py as f64 + (sy as f64 + 0.5) / samples as f64) * 256. / size as f64;
                    let u = 128. + (x - 128.) / 0.94 - 0.42 * (128. - y);
                    if mask(u, y) {
                        let edge = [(u - 3., y), (u + 3., y), (u, y - 3.), (u, y + 3.)]
                            .iter()
                            .any(|&(a, b)| !mask(a, b));
                        let bevel = !edge
                            && [(u - 5., y - 5.), (u + 5., y + 5.)]
                                .iter()
                                .any(|&(a, b)| !mask(a, b));
                        let c = if edge || separator(u, y) {
                            [7., 18., 51.]
                        } else if bevel {
                            [178., 205., 233.]
                        } else {
                            color(y)
                        };
                        for i in 0..3 {
                            sum[i] += c[i];
                        }
                        sum[3] += 1.;
                    }
                }
            }
            for value in &sum[..3] {
                out.push(if sum[3] > 0. {
                    (value / sum[3]).round() as u8
                } else {
                    0
                });
            }
            out.push((sum[3] * 255. / (samples * samples) as f64).round() as u8);
        }
    }
    out
}
#[cfg(test)]
mod tests {
    #[test]
    fn separator_stays_off_arrows() {
        assert!(super::separator(128.0, 128.0));
        assert!(!super::separator(128.0, 60.0));
        assert!(!super::separator(128.0, 196.0));
    }
    #[test]
    fn bowls_touch_without_a_gap() {
        for x in 120..136 {
            assert!(super::mask(x as f64 + 0.5, 128.0));
        }
    }
    #[test]
    fn letters_are_exact_rotations_after_common_slant() {
        for size in [16, 32, 64, 256] {
            let data = super::rgba(size);
            for i in 0..(size * size) as usize {
                assert_eq!(
                    data[i * 4 + 3],
                    data[data.len() - 1 - i * 4],
                    "alpha rotational symmetry at {size}"
                );
            }
        }
    }
}
