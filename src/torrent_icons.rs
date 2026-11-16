// Resource IDs are stable: 1 remains the application logo, 2 the default magnet.
pub struct FileIcon {
    pub resource: u32,
    pub title: &'static str,
    png: &'static [u8],
}

pub const ICONS: &[FileIcon] = &[
    FileIcon {
        resource: 2,
        title: "Объёмный магнит",
        png: include_bytes!("../assets/file-icons/magnet.png"),
    },
    FileIcon {
        resource: 3,
        title: "Документ Torrent",
        png: include_bytes!("../assets/file-icons/document.png"),
    },
    FileIcon {
        resource: 4,
        title: "Синяя стрелка",
        png: include_bytes!("../assets/file-icons/blue-arrow.png"),
    },
    FileIcon {
        resource: 5,
        title: "Зелёная стрелка",
        png: include_bytes!("../assets/file-icons/green-arrow.png"),
    },
    FileIcon {
        resource: 6,
        title: "Зелёная плитка",
        png: include_bytes!("../assets/file-icons/green-tile.png"),
    },
    FileIcon {
        resource: 7,
        title: "Синий круг",
        png: include_bytes!("../assets/file-icons/blue-circle.png"),
    },
];

impl FileIcon {
    pub fn decode(&self) -> image::RgbaImage {
        image::load_from_memory(self.png)
            .unwrap_or_else(|e| panic!("{}: {e}", self.title))
            .into_rgba8()
    }
}

pub fn resize(source: &image::RgbaImage, size: u32) -> Vec<u8> {
    // Filter premultiplied colours to avoid dark fringes on transparent edges.
    let mut premultiplied = source.clone();
    for p in premultiplied.pixels_mut() {
        for c in 0..3 {
            p[c] = ((p[c] as u16 * p[3] as u16 + 127) / 255) as u8;
        }
    }
    let mut result = image::imageops::resize(
        &premultiplied,
        size,
        size,
        image::imageops::FilterType::Lanczos3,
    );
    for p in result.pixels_mut() {
        for c in 0..3 {
            p[c] = (u32::from(p[c]) * 255)
                .checked_div(u32::from(p[3]))
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
    result.into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_shell_icons_have_unique_ids_and_transparent_small_sizes() {
        let mut ids = std::collections::HashSet::new();
        let mut images = std::collections::HashSet::new();
        for entry in ICONS {
            assert!(entry.resource > 1 && ids.insert(entry.resource));
            let source = entry.decode();
            assert_eq!(source.width(), source.height());
            for size in [16, 32, 48, 256] {
                let rgba = resize(&source, size);
                assert_eq!(rgba.len(), (size * size * 4) as usize);
                assert!(rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
                assert!(rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 255));
                assert!(images.insert(rgba));
            }
        }
    }
}
