//! Synthetic documentation fixture: recording starts during an existing download.
pub const SIZE: u64 = 6_000_000_000;
pub const DOWNLOADED: u64 = 3_840_000_000;
pub const UPLOADED: u64 = 3_000_000_000;
pub const DOWN: u64 = 12_450_000;
pub const UP: u64 = 6_600_000;
pub const SEED_UP: u64 = 2_100_000;
pub const PROGRESS: f32 = DOWNLOADED as f32 / SIZE as f32;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub seconds: f64,
    pub down: u64,
    pub up: u64,
    pub seeding: u64,
}
pub fn frames() -> Vec<Frame> {
    // Irregular peer demand, with short dips and recoveries. No periodic wave.
    let demand = [
        (0, 11.9),
        (3, 12.4),
        (5, 6.8),
        (6, 5.0),
        (8, 11.7),
        (13, 11.9),
        (15, 4.4),
        (17, 12.0),
        (22, 12.5),
        (23, 7.0),
        (25, 9.2),
        (27, 6.8),
        (29, 11.4),
        (33, 12.2),
        (37, 9.4),
        (38, 6.3),
        (40, 12.1),
        (43, 12.1),
        (44, 9.5),
        (46, 4.8),
        (47, 5.2),
        (48, 10.4),
        (49, 6.0),
        (50, 11.0),
        (52, 5.0),
        (53, 5.4),
        (55, 11.9),
        (58, 12.0),
        (61, 10.2),
        (63, 4.6),
        (66, 11.5),
        (69, 12.2),
        (70, 7.4),
        (72, 6.8),
        (74, 11.2),
        (77, 11.7),
        (78, 6.0),
        (80, 6.6),
    ];
    let mut seed = 0x51a7_92d3_u32;
    let mut noise = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        f64::from(seed) / f64::from(u32::MAX) - 0.5
    };
    let mut result = Vec::new();
    for i in 0..=80 {
        let segment = demand.windows(2).find(|p| i <= p[1].0).unwrap();
        let t = (i - segment[0].0) as f64 / (segment[1].0 - segment[0].0) as f64;
        let up = (segment[0].1 * (1.0 - t) + segment[1].1 * t) * 1_000_000.0;
        result.push(Frame {
            seconds: i as f64 * 3.0,
            down: (DOWN as f64 + noise() * 1_100_000.0) as u64,
            up: (up + noise() * 650_000.0) as u64,
            seeding: (SEED_UP as f64 + noise() * 300_000.0) as u64,
        });
    }
    let last = result.last_mut().unwrap();
    last.down = DOWN;
    last.up = UP;
    last.seeding = SEED_UP;
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traffic_is_repeatable_and_consistent_with_already_received_pieces() {
        let frames = frames();
        assert_eq!(frames, super::frames());
        assert_eq!(frames.last().unwrap().seconds, 240.0);
        let down: f64 = frames
            .windows(2)
            .map(|p| (p[0].down + p[1].down) as f64 * 0.5 * 3.0)
            .sum();
        let up: f64 = frames
            .windows(2)
            .map(|p| (p[0].up + p[1].up) as f64 * 0.5 * 3.0)
            .sum();
        let received_before = DOWNLOADED as f64 - down;
        assert!(received_before > 500_000_000.0 && received_before < 1_200_000_000.0);
        assert!(UPLOADED as f64 > up);
        assert!(frames
            .iter()
            .all(|f| f.down > 11_800_000 && f.down < 13_100_000));
        assert!(frames.iter().any(|f| f.up < 5_000_000));
        assert!(frames.iter().any(|f| f.up > 12_000_000));
        assert_eq!(
            (
                frames.last().unwrap().down,
                frames.last().unwrap().up,
                frames.last().unwrap().seeding
            ),
            (DOWN, UP, SEED_UP)
        );
        let mut downloaded = received_before;
        let mut uploaded = 0.0;
        for p in frames.windows(2) {
            downloaded += (p[0].down + p[1].down) as f64 * 1.5;
            uploaded += (p[0].up + p[1].up) as f64 * 1.5;
            assert!(uploaded <= downloaded);
        }
        assert!((downloaded / SIZE as f64 - PROGRESS as f64).abs() < 0.000001);
    }
}
