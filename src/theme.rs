use eframe::egui::{self, Color32};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Marine,
    White,
    #[default]
    #[serde(other)]
    Graphite,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub muted: Color32,
    pub accent: Color32,
    pub blue: Color32,
    pub green: Color32,
    pub orange: Color32,
    pub red: Color32,
    pub purple: Color32,
    pub surface: Color32,
    pub sidebar: Color32,
    pub window: Color32,
    pub input: Color32,
    pub stripe: Color32,
    pub text: Color32,
    pub button: Color32,
    pub hover: Color32,
    pub active: Color32,
    pub border: Color32,
    pub selection: Color32,
    pub badge: Color32,
    pub logo: Color32,
}

impl Theme {
    pub const ALL: [Self; 3] = [Self::Graphite, Self::Marine, Self::White];

    pub fn name(self) -> &'static str {
        match self {
            Self::Marine => "Marine Dark Theme",
            Self::Graphite => "Graphite Theme",
            Self::White => "White Theme",
        }
    }

    pub fn palette(self) -> Palette {
        let c = Color32::from_rgb;
        match self {
            Self::White => Palette {
                muted: c(105, 110, 118),
                accent: c(48, 108, 151),
                blue: c(49, 105, 166),
                green: c(49, 117, 78),
                orange: c(148, 96, 38),
                red: c(174, 65, 81),
                purple: c(118, 82, 157),
                surface: c(237, 238, 240),
                sidebar: c(227, 229, 232),
                window: c(242, 243, 245),
                input: c(225, 228, 232),
                stripe: c(231, 233, 236),
                text: c(64, 69, 77),
                button: c(220, 223, 228),
                hover: c(211, 219, 228),
                active: c(199, 213, 228),
                border: c(202, 207, 214),
                selection: c(207, 222, 237),
                badge: c(214, 220, 228),
                logo: c(240, 242, 245),
            },
            Self::Marine => Palette {
                muted: c(146, 163, 184),
                accent: c(64, 199, 191),
                blue: c(84, 169, 255),
                green: c(87, 211, 150),
                orange: c(255, 181, 93),
                red: c(249, 113, 132),
                purple: c(182, 153, 255),
                surface: c(24, 33, 46),
                sidebar: c(18, 27, 39),
                window: c(30, 41, 57),
                input: c(16, 24, 35),
                stripe: c(28, 39, 54),
                text: c(226, 228, 231),
                button: c(36, 49, 66),
                hover: c(44, 65, 84),
                active: c(37, 84, 96),
                border: c(44, 59, 77),
                selection: c(32, 68, 83),
                badge: c(40, 54, 71),
                logo: c(13, 43, 48),
            },
            Self::Graphite => Palette {
                muted: c(160, 160, 166),
                accent: c(64, 196, 255),
                blue: c(73, 157, 255),
                green: c(67, 215, 132),
                orange: c(255, 174, 66),
                red: c(255, 100, 119),
                purple: c(180, 132, 255),
                surface: c(31, 31, 33),
                sidebar: c(25, 25, 27),
                window: c(38, 38, 41),
                input: c(23, 23, 25),
                stripe: c(36, 36, 39),
                text: c(229, 229, 231),
                button: c(47, 47, 51),
                hover: c(57, 57, 62),
                active: c(66, 66, 72),
                border: c(59, 59, 64),
                selection: c(63, 66, 73),
                badge: c(46, 46, 50),
                logo: c(32, 33, 36),
            },
        }
    }

    pub fn apply(self, ctx: &egui::Context) {
        let p = self.palette();
        let mut v = if self == Self::White {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        v.panel_fill = p.surface;
        v.window_fill = p.window;
        v.extreme_bg_color = p.input;
        v.faint_bg_color = p.stripe;
        v.override_text_color = Some(p.text);
        v.widgets.inactive.bg_fill = p.button;
        v.widgets.hovered.bg_fill = p.hover;
        v.widgets.active.bg_fill = p.active;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, p.border);
        v.widgets.noninteractive.fg_stroke.color = p.muted;
        v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        v.window_stroke = egui::Stroke::new(1.0_f32, p.border);
        v.hyperlink_color = p.blue;
        v.error_fg_color = p.red;
        v.warn_fg_color = p.orange;
        v.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        v.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        v.widgets.active.corner_radius = egui::CornerRadius::same(6);
        v.selection.bg_fill = p.selection;
        v.selection.stroke = egui::Stroke::new(1.0_f32, p.text);
        ctx.set_visuals(v);
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("app-theme"), self));
    }
}

pub fn current(ctx: &egui::Context) -> Theme {
    ctx.data(|data| data.get_temp(egui::Id::new("app-theme")))
        .unwrap_or_default()
}

pub fn palette(ctx: &egui::Context) -> Palette {
    current(ctx).palette()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_survives_restart_and_missing_theme_defaults_to_graphite() {
        let old: crate::model::Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.theme, Theme::Graphite);
        for theme in Theme::ALL {
            let settings = crate::model::Settings {
                theme,
                ..Default::default()
            };
            let saved = ron::to_string(&settings).unwrap();
            let restored: crate::model::Settings = ron::from_str(&saved).unwrap();
            assert_eq!(restored.theme, theme);
        }
    }

    #[test]
    fn switching_replaces_surfaces_and_keeps_text_readable() {
        let ctx = egui::Context::default();
        for theme in [
            Theme::Marine,
            Theme::White,
            Theme::Graphite,
            Theme::White,
            Theme::Marine,
        ] {
            theme.apply(&ctx);
            let p = theme.palette();
            assert_eq!(current(&ctx), theme);
            assert_eq!(ctx.style().visuals.panel_fill, p.surface);
            assert_eq!(ctx.style().visuals.selection.bg_fill, p.selection);
            assert!(p.text.r().abs_diff(p.surface.r()) > 150);
            assert_eq!(ctx.style().visuals.dark_mode, theme != Theme::White);
        }
    }
}
