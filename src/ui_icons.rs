//! Filled, semantic action pictograms with restrained highlights and relief.
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

pub const PLAY: usize = 13;
pub const VERIFY: usize = 14;
pub const TRACKERS: usize = 15;
pub const TOP: usize = 16;
pub const BOTTOM: usize = 17;
pub const EDGES: usize = 18;
pub const FORCE: usize = 19;
pub const AUTO: usize = 20;
pub const SEED: usize = 21;
pub const RENAME: usize = 22;
pub const CATEGORY: usize = 23;
pub const ADD_TAG: usize = 24;
pub const REMOVE_TAG: usize = 25;
pub const COMMENT: usize = 26;
pub const MOVE: usize = 27;
pub const DOWNLOAD_LIMIT: usize = 28;
pub const UPLOAD_LIMIT: usize = 29;
pub const SHARE_LIMIT: usize = 30;
pub const EXPORT: usize = 31;
pub const ERROR: usize = 32;

pub fn color(kind: usize, p: crate::theme::Palette) -> Color32 {
    match kind {
        1 | 4 | BOTTOM | REMOVE_TAG | DOWNLOAD_LIMIT | ERROR => p.red,
        2 | PLAY | TOP | FORCE | SEED | ADD_TAG | UPLOAD_LIMIT => p.green,
        3 | 7 | 10 | 11 | CATEGORY | RENAME | MOVE | SHARE_LIMIT => p.orange,
        5 | 6 | VERIFY | TRACKERS | EXPORT => p.blue,
        _ => p.muted,
    }
}

struct Artist<'a> {
    p: &'a egui::Painter,
    center: Pos2,
}
impl Artist<'_> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.center + egui::vec2(x, y)
    }
    fn rect(&self, a: (f32, f32), b: (f32, f32), fill: Color32) {
        let rect = Rect::from_min_max(self.at(a.0, a.1), self.at(b.0, b.1));
        self.p.rect_filled(
            rect.translate(egui::vec2(0.0, 0.8)),
            1.2,
            Color32::from_black_alpha(55),
        );
        self.p.rect_filled(rect, 1.2, fill);
        self.p.rect_stroke(
            rect,
            1.2,
            Stroke::new(0.65_f32, fill.gamma_multiply(0.65)),
            egui::StrokeKind::Inside,
        );
        self.line(
            (a.0 + 1.0, a.1 + 0.8),
            (b.0 - 1.0, a.1 + 0.8),
            Color32::from_white_alpha(85),
            0.7,
        );
    }
    fn poly(&self, points: &[(f32, f32)], fill: Color32) {
        let vertices: Vec<_> = points.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.p.add(egui::Shape::convex_polygon(
            vertices.iter().map(|p| *p + egui::vec2(0.0, 0.8)).collect(),
            Color32::from_black_alpha(55),
            Stroke::NONE,
        ));
        self.p.add(egui::Shape::convex_polygon(
            vertices,
            fill,
            Stroke::new(0.65_f32, fill.gamma_multiply(0.65)),
        ));
    }
    fn line(&self, a: (f32, f32), b: (f32, f32), color: Color32, width: f32) {
        self.p.line_segment(
            [self.at(a.0, a.1), self.at(b.0, b.1)],
            Stroke::new(width, color),
        );
    }
    fn dot(&self, x: f32, y: f32, r: f32, fill: Color32) {
        self.p
            .circle_filled(self.at(x, y + 0.6), r, Color32::from_black_alpha(50));
        self.p.circle_filled(self.at(x, y), r, fill);
        self.p.circle_stroke(
            self.at(x, y),
            r,
            Stroke::new(0.65_f32, fill.gamma_multiply(0.6)),
        );
    }
    fn arrow(&self, down: bool, fill: Color32) {
        let d = if down { 1.0 } else { -1.0 };
        // A broad shaft and faceted head remain distinct at 16 logical pixels.
        self.rect(
            (-2.0, if down { -6.0 } else { -3.0 }),
            (2.0, if down { 3.0 } else { 6.0 }),
            fill,
        );
        self.poly(&[(-6.0, 0.0 * d), (6.0, 0.0 * d), (0.0, 6.8 * d)], fill);
        self.line(
            (-1.0, -5.0),
            (-1.0, if down { 2.0 } else { -1.0 }),
            Color32::from_white_alpha(115),
            0.8,
        );
    }
    fn page(&self) {
        self.rect((-5.5, -7.0), (5.0, 6.5), Color32::from_rgb(231, 236, 242));
        self.poly(
            &[(1.0, -7.0), (5.0, -3.0), (1.0, -3.0)],
            Color32::from_rgb(151, 178, 205),
        );
    }
    fn list(&self, edge: bool) {
        self.rect((-6.5, -6.0), (6.5, 6.0), Color32::from_rgb(163, 173, 186));
        for y in [-3.5, 0.0, 3.5] {
            self.dot(-4.0, y, 0.65, Color32::from_rgb(70, 82, 98));
            self.line(
                (-1.8, y),
                (4.2, y),
                if edge && y != 0.0 {
                    Color32::from_rgb(52, 128, 205)
                } else {
                    Color32::from_rgb(76, 87, 101)
                },
                1.25,
            );
        }
    }
    fn tag(&self) {
        self.poly(
            &[
                (-6.5, -5.0),
                (0.0, -5.0),
                (7.0, 1.0),
                (0.0, 7.0),
                (-6.5, 1.0),
            ],
            Color32::from_rgb(238, 183, 66),
        );
        self.dot(-3.8, -2.3, 1.2, Color32::from_rgb(108, 83, 40));
        self.line(
            (-1.0, -3.5),
            (4.0, 1.0),
            Color32::from_white_alpha(110),
            0.8,
        );
    }
    fn badge(&self, plus: bool, fill: Color32) {
        self.dot(4.0, 3.7, 3.5, fill);
        self.line((1.8, 3.7), (6.2, 3.7), Color32::WHITE, 1.25);
        if plus {
            self.line((4.0, 1.5), (4.0, 5.9), Color32::WHITE, 1.25);
        }
    }
    fn gear(&self, fill: Color32) {
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            let c = self.at(a.cos() * 5.3, a.sin() * 5.3);
            self.p
                .rect_filled(Rect::from_center_size(c, egui::vec2(3.0, 3.0)), 0.5, fill);
        }
        self.dot(0.0, 0.0, 5.5, fill);
        self.dot(0.0, 0.0, 2.3, Color32::from_rgb(52, 111, 164));
        self.p.circle_stroke(
            self.at(0.0, 0.0),
            3.4,
            Stroke::new(0.7_f32, Color32::from_white_alpha(100)),
        );
    }
}

pub fn draw(p: &egui::Painter, center: Pos2, kind: usize, _fallback: Color32) {
    let palette = crate::theme::palette(p.ctx());
    let c = color(kind, palette);
    let a = Artist { p, center };
    let gray = Color32::from_rgb(155, 167, 181);
    match kind {
        0 | 8 => a.list(false),
        1 | 2 | TOP | BOTTOM | DOWNLOAD_LIMIT | UPLOAD_LIMIT => {
            a.arrow(matches!(kind, 1 | BOTTOM | DOWNLOAD_LIMIT), c);
            if matches!(kind, TOP | UPLOAD_LIMIT) {
                a.line((-6.0, -7.2), (6.0, -7.2), gray, 1.3);
            }
            if matches!(kind, BOTTOM | DOWNLOAD_LIMIT) {
                a.line((-6.0, 7.2), (6.0, 7.2), gray, 1.3);
            }
        }
        PLAY => {
            a.poly(&[(-4.5, -6.5), (6.0, 0.0), (-4.5, 6.5)], palette.green);
            a.line(
                (-3.5, -4.4),
                (-3.5, 3.7),
                Color32::from_white_alpha(110),
                0.8,
            );
        }
        3 => {
            for x in [-5.0, 1.0] {
                a.rect((x, -6.0), (x + 4.0, 6.0), palette.orange);
            }
        }
        ERROR => {
            a.poly(&[(0.0, -7.0), (7.0, 6.0), (-7.0, 6.0)], palette.red);
            a.line((0.0, -2.7), (0.0, 1.3), Color32::WHITE, 1.7);
            a.dot(0.0, 3.8, 0.9, Color32::WHITE);
        }
        4 => {
            a.rect((-4.7, -3.5), (4.7, 6.3), palette.red);
            a.rect((-6.0, -5.5), (6.0, -3.5), palette.red);
            a.line((-2.0, -7.0), (2.0, -7.0), palette.red, 1.8);
            for x in [-2.0, 1.5] {
                a.line((x, -1.5), (x, 4.5), Color32::from_white_alpha(155), 0.9);
            }
        }
        5 => {
            a.rect((-7.0, -6.0), (7.0, 6.0), gray);
            for (x, y) in [(-4.8, 2.5), (-1.0, -1.5), (2.0, 1.0), (5.0, -3.7)]
                .windows(2)
                .map(|s| (s[0], s[1]))
            {
                a.line(x, y, palette.blue, 1.8);
            }
        }
        6 => {
            a.dot(-1.5, -1.5, 5.0, palette.blue);
            p.circle_filled(a.at(-1.5, -1.5), 3.4, Color32::from_rgb(193, 229, 246));
            a.line((2.2, 2.2), (6.2, 6.2), Color32::from_rgb(78, 100, 129), 3.0);
            a.line(
                (-3.6, -3.2),
                (-1.8, -4.0),
                Color32::from_white_alpha(200),
                1.0,
            );
        }
        7 => {
            a.rect((-7.0, -7.0), (7.0, 7.0), Color32::from_rgb(232, 150, 47));
            a.dot(-3.5, 3.5, 1.3, Color32::WHITE);
            for r in [4.0_f32, 7.5] {
                let points = (0..12)
                    .map(|i| {
                        let t = i as f32 * std::f32::consts::FRAC_PI_2 / 11.0;
                        a.at(-3.5 + r * t.sin(), 3.5 - r * t.cos())
                    })
                    .collect();
                p.add(egui::Shape::line(
                    points,
                    Stroke::new(1.6_f32, Color32::WHITE),
                ));
            }
        }
        9 | AUTO => a.gear(gray),
        10 | ADD_TAG | REMOVE_TAG => {
            a.tag();
            if kind == ADD_TAG {
                a.badge(true, palette.green);
            }
            if kind == REMOVE_TAG {
                a.badge(false, palette.red);
            }
        }
        11 | CATEGORY | MOVE => {
            crate::file_icons::paint_open_folder(
                p,
                Rect::from_center_size(center, egui::vec2(16.0, 13.0)),
            );
            if kind == MOVE {
                a.line((-0.5, 3.5), (6.5, 3.5), palette.green, 2.0);
                a.line((3.5, 0.5), (6.5, 3.5), palette.green, 2.0);
                a.line((3.5, 6.5), (6.5, 3.5), palette.green, 2.0);
            }
        }
        12 | VERIFY | EXPORT => {
            a.page();
            if kind == VERIFY {
                a.line((-3.0, 0.3), (-0.2, 3.0), palette.green, 2.0);
                a.line((-0.2, 3.0), (5.8, -3.0), palette.green, 2.0);
            } else if kind == EXPORT {
                a.line((3.0, -1.0), (3.0, 6.0), palette.red, 1.8);
                a.line((0.0, 3.0), (3.0, 6.0), palette.red, 1.8);
                a.line((6.0, 3.0), (3.0, 6.0), palette.red, 1.8);
            } else {
                for y in [-1.5, 1.2, 3.9] {
                    a.line((-3.5, y), (2.7, y), gray, 0.85);
                }
            }
        }
        TRACKERS => {
            for y in [-5.0, 1.5] {
                a.rect((-6.5, y), (6.5, y + 4.5), Color32::from_rgb(103, 145, 186));
                a.dot(-4.5, y + 2.2, 0.8, palette.green);
                a.line(
                    (-1.5, y + 2.2),
                    (4.8, y + 2.2),
                    Color32::from_rgb(230, 241, 249),
                    0.85,
                );
            }
        }
        EDGES => a.list(true),
        FORCE => {
            a.poly(
                &[(0.0, -7.5), (5.8, -7.5), (-0.3, 0.5), (-5.3, 0.5)],
                palette.green,
            );
            a.poly(&[(-0.3, -0.5), (5.3, -0.5), (-2.5, 7.5)], palette.green);
        }
        SEED => {
            for (x, y) in [(-5.0, 4.0), (5.0, 4.0)] {
                a.line((0.0, -4.5), (x, y), gray, 1.5);
            }
            for (x, y) in [(0.0, -4.5), (-5.0, 4.0), (5.0, 4.0)] {
                a.dot(x, y, 2.4, palette.green);
            }
        }
        RENAME => {
            a.poly(
                &[(-5.5, 2.0), (2.8, -6.3), (6.3, -2.8), (-2.0, 5.5)],
                Color32::from_rgb(239, 186, 67),
            );
            a.poly(
                &[(-5.5, 2.0), (-2.0, 5.5), (-7.0, 7.0)],
                Color32::from_rgb(219, 203, 172),
            );
            a.poly(
                &[(-6.4, 5.3), (-5.3, 6.4), (-7.0, 7.0)],
                Color32::from_rgb(67, 73, 81),
            );
            a.line(
                (-3.5, 2.0),
                (3.4, -4.9),
                Color32::from_white_alpha(150),
                0.8,
            );
        }
        COMMENT => {
            a.rect((-7.0, -5.5), (7.0, 3.5), Color32::from_rgb(196, 205, 215));
            a.poly(
                &[(-4.0, 2.5), (0.5, 2.5), (-4.0, 7.0)],
                Color32::from_rgb(196, 205, 215),
            );
            for y in [-2.0, 0.7] {
                a.line((-4.5, y), (4.5, y), Color32::from_rgb(90, 105, 122), 1.0);
            }
        }
        SHARE_LIMIT => {
            a.dot(0.0, 0.0, 6.5, gray);
            p.circle_filled(center, 4.9, Color32::from_rgb(237, 222, 183));
            a.line((0.0, -3.5), (0.0, 0.0), Color32::from_rgb(89, 97, 112), 1.5);
            a.line((0.0, 0.0), (3.1, 1.8), Color32::from_rgb(89, 97, 112), 1.5);
        }
        _ => a.list(false),
    }
}
