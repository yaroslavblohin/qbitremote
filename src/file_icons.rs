use eframe::egui::{self, TextureHandle};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

/// Compact open folder with a raised tab, paper inserts and a slanted front.
/// All coordinates are logical points so the silhouette stays crisp at HiDPI.
pub fn paint_open_folder(painter: &egui::Painter, rect: egui::Rect) {
    let scale = (rect.width() / 20.0).min(rect.height() / 16.0);
    let origin = rect.center() - egui::vec2(10.0, 8.0) * scale;
    let point = |x, y| origin + egui::vec2(x, y) * scale;
    let back = egui::Color32::from_rgb(234, 174, 0);
    let front = egui::Color32::from_rgb(255, 201, 0);
    let polygon = |coords: &[(f32, f32)], color| {
        painter.add(egui::Shape::convex_polygon(
            coords.iter().map(|&(x, y)| point(x, y)).collect(),
            color,
            egui::Stroke::NONE,
        ));
    };
    painter.rect_filled(
        egui::Rect::from_min_max(point(0.0, 0.0), point(7.0, 6.0)),
        scale,
        back,
    );
    polygon(&[(6.0, 0.0), (7.0, 0.0), (10.0, 2.1), (6.0, 3.0)], back);
    painter.rect_filled(
        egui::Rect::from_min_max(point(0.0, 2.0), point(17.0, 16.0)),
        scale,
        back,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(point(1.5, 3.2), point(15.8, 13.5)),
        scale * 0.7,
        egui::Color32::from_rgb(224, 226, 230),
    );
    painter.rect_filled(
        egui::Rect::from_min_max(point(2.0, 3.9), point(16.3, 14.0)),
        scale * 0.7,
        egui::Color32::from_rgb(250, 250, 248),
    );
    polygon(
        &[
            (3.3, 5.7),
            (18.7, 5.7),
            (16.5, 14.9),
            (15.6, 16.0),
            (1.0, 16.0),
            (0.2, 15.5),
            (0.0, 14.7),
            (2.4, 6.6),
        ],
        front,
    );
    polygon(
        &[
            (9.5, 5.8),
            (12.8, 4.2),
            (18.4, 4.2),
            (19.1, 4.5),
            (19.4, 5.2),
            (18.8, 7.0),
            (9.5, 7.0),
        ],
        front,
    );
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Key {
    extension: String,
    pixels: u32,
}
impl Key {
    fn new(name: &str, scale: f32) -> Self {
        let extension = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_lowercase())
            .filter(|s| {
                s.len() <= 32
                    && s.chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
            })
            .unwrap_or_default();
        let wanted = (16.0 * scale).ceil() as u32;
        Self {
            extension,
            pixels: [16, 24, 32, 48, 64]
                .into_iter()
                .find(|n| *n >= wanted)
                .unwrap_or(64),
        }
    }
}
struct Cache {
    textures: HashMap<Key, Option<TextureHandle>>,
    requests: mpsc::Sender<(Key, egui::Context)>,
    results: mpsc::Receiver<(Key, Option<egui::ColorImage>)>,
}
impl Cache {
    fn new() -> Self {
        let (requests, receiver) = mpsc::channel::<(Key, egui::Context)>();
        let (sender, results) = mpsc::channel();
        std::thread::spawn(move || {
            for (key, ctx) in receiver {
                let image = native::load(&key);
                if sender.send((key, image)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            textures: HashMap::new(),
            requests,
            results,
        }
    }
    fn texture(&mut self, mut key: Key, ctx: &egui::Context) -> Option<TextureHandle> {
        while let Ok((key, image)) = self.results.try_recv() {
            let texture = image.map(|image| {
                ctx.load_texture(
                    format!("shell-icon-{key:?}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
            self.textures.insert(key, texture);
        }
        // Unusual torrents can contain thousands of distinct suffixes. Keep a
        // bounded set and use the system's generic file icon past the limit.
        if self.textures.len() >= 512 && !self.textures.contains_key(&key) {
            key.extension.clear();
        }
        if !self.textures.contains_key(&key) {
            self.textures.insert(key.clone(), None);
            let _ = self.requests.send((key.clone(), ctx.clone()));
        }
        self.textures.get(&key).cloned().flatten()
    }
}

/// Folder Some(open), file None. Shell lookup uses only the extension/attributes;
/// it never reads torrent contents, mounted paths, or executable resources.
pub fn show(ui: &mut egui::Ui, name: &str, folder: Option<bool>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
    if let Some(open) = folder {
        let p = |x, y| rect.center() + egui::vec2(x, y);
        let gold = egui::Color32::from_rgb(244, 190, 69);
        let edge = egui::Stroke::new(0.8_f32, egui::Color32::from_rgb(186, 135, 38));
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                p(-7.0, -5.5),
                p(-2.0, -5.5),
                p(0.0, -3.5),
                p(7.0, -3.5),
                p(7.0, 6.0),
                p(-7.0, 6.0),
            ],
            egui::Color32::from_rgb(216, 157, 45),
            edge,
        ));
        if open {
            ui.painter().add(egui::Shape::convex_polygon(
                vec![p(-5.0, -1.0), p(8.0, -1.0), p(5.0, 6.0), p(-8.0, 6.0)],
                gold,
                edge,
            ));
        } else {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(p(-7.0, -2.0), p(7.0, 6.0)),
                1.0,
                gold,
            );
            ui.painter().line_segment(
                [p(-6.0, -1.0), p(6.0, -1.0)],
                egui::Stroke::new(0.8_f32, egui::Color32::from_rgb(255, 216, 114)),
            );
        }
    } else {
        let key = Key::new(name, ui.ctx().pixels_per_point());
        let cache = ui.ctx().data_mut(|data| {
            data.get_temp_mut_or_insert_with(egui::Id::new("windows-file-icons"), || {
                Arc::new(Mutex::new(Cache::new()))
            })
            .clone()
        });
        let texture = cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .texture(key, ui.ctx());
        if let Some(texture) = texture {
            ui.painter().image(
                texture.id(),
                rect.shrink(1.0),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            crate::app::draw_icon(
                ui.painter(),
                rect.center(),
                12,
                crate::theme::palette(ui.ctx()).muted,
            );
        }
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            match folder {
                Some(true) => "Свернуть папку",
                Some(false) => "Раскрыть папку",
                None => name,
            },
        )
    });
    response
}

#[cfg(windows)]
mod native {
    use super::*;
    use windows_sys::Win32::{
        Graphics::Gdi::*,
        Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
        System::Com::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    };
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    pub(super) fn load(key: &Key) -> Option<egui::ColorImage> {
        if unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) } < 0 {
            return None;
        }
        let _com = Com;
        let name = if key.extension.is_empty() {
            "qbit-icon".into()
        } else {
            format!("qbit-icon.{}", key.extension)
        };
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let flags = SHGFI_ICON
            | SHGFI_USEFILEATTRIBUTES
            | if key.pixels <= 16 {
                SHGFI_SMALLICON
            } else {
                SHGFI_LARGEICON
            };
        let result = unsafe {
            SHGetFileInfoW(
                wide.as_ptr(),
                FILE_ATTRIBUTE_NORMAL,
                &mut info,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };
        if result == 0 || info.hIcon.is_null() {
            return None;
        }
        let black = render(info.hIcon, key.pixels, 0);
        let white = render(info.hIcon, key.pixels, 255);
        unsafe {
            DestroyIcon(info.hIcon);
        }
        let (black, white) = (black?, white?);
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [key.pixels as usize; 2],
            &rgba(&black, &white),
        ))
    }
    fn render(icon: HICON, size: u32, background: u8) -> Option<Vec<u8>> {
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = size as i32;
            info.bmiHeader.biHeight = -(size as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                dc,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() {
                DeleteDC(dc);
                return None;
            }
            let old = SelectObject(dc, bitmap);
            let len = (size * size * 4) as usize;
            std::ptr::write_bytes(bits, background, len);
            let result = DrawIconEx(
                dc,
                0,
                0,
                icon,
                size as i32,
                size as i32,
                0,
                std::ptr::null_mut(),
                DI_NORMAL,
            );
            GdiFlush();
            let pixels =
                (result != 0).then(|| std::slice::from_raw_parts(bits.cast::<u8>(), len).to_vec());
            SelectObject(dc, old);
            DeleteObject(bitmap);
            DeleteDC(dc);
            pixels
        }
    }
    /// Recover alpha from two backgrounds. Handles alpha icons and legacy AND
    /// masks alike, including white documents on the application's dark theme.
    fn rgba(black: &[u8], white: &[u8]) -> Vec<u8> {
        black
            .as_chunks::<4>()
            .0
            .iter()
            .zip(white.as_chunks::<4>().0.iter())
            .flat_map(|(b, w)| {
                let alpha = 255
                    - (0..3)
                        .map(|i| w[i].saturating_sub(b[i]))
                        .max()
                        .unwrap_or(255);
                let channel = |i| {
                    if alpha == 0 {
                        0
                    } else {
                        (u32::from(b[i]) * 255 / u32::from(alpha)).min(255) as u8
                    }
                };
                [channel(2), channel(1), channel(0), alpha]
            })
            .collect()
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn alpha_handles_transparent_opaque_and_translucent_icons() {
            assert_eq!(
                rgba(
                    &[0, 0, 0, 0, 10, 20, 30, 0, 0, 0, 128, 0],
                    &[255, 255, 255, 0, 10, 20, 30, 0, 127, 127, 255, 0]
                ),
                vec![0, 0, 0, 0, 30, 20, 10, 255, 255, 0, 0, 128]
            );
        }
        #[test]
        fn windows_returns_file_type_icons_at_hidpi_sizes() {
            for scale in [1.0, 1.5, 2.0, 3.0] {
                for name in ["sample.txt", "sample.zip", "sample.mkv", "file"] {
                    let key = Key::new(name, scale);
                    let image = load(&key)
                        .expect("Windows Shell must return a type icon without an existing file");
                    assert_eq!(image.size, [key.pixels as usize; 2]);
                    assert!(image.pixels.iter().any(|p| p.a() > 0));
                    assert!(image.pixels.iter().any(|p| p.a() == 0));
                }
            }
        }
    }
}
#[cfg(not(windows))]
mod native {
    use super::*;
    pub(super) fn load(_: &Key) -> Option<egui::ColorImage> {
        None
    }
}
