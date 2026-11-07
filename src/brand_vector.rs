//! Font-independent SVG artwork rasterized directly at the requested pixel size.
use std::sync::OnceLock;
const SVG: &[u8] = include_bytes!("../assets/brand-lockup.svg");
fn tree() -> &'static resvg::usvg::Tree {
    static TREE: OnceLock<resvg::usvg::Tree> = OnceLock::new();
    TREE.get_or_init(|| {
        resvg::usvg::Tree::from_data(SVG, &resvg::usvg::Options::default())
            .expect("Invalid embedded vector logo")
    })
}
pub fn aspect() -> f32 {
    tree().size().width() / tree().size().height()
}
pub fn raster(width: u32, height: u32) -> image::RgbaImage {
    let mut pixels = resvg::tiny_skia::Pixmap::new(width, height).expect("Invalid logo dimensions");
    let transform = resvg::tiny_skia::Transform::from_scale(
        width as f32 / tree().size().width(),
        height as f32 / tree().size().height(),
    );
    resvg::render(tree(), transform, &mut pixels.as_mut());
    let mut data = pixels.take();
    // tiny-skia is premultiplied; PNG and egui's unmultiplied loader need straight RGBA.
    for pixel in data.as_chunks_mut::<4>().0 {
        let alpha = pixel[3] as u32;
        for channel in &mut pixel[..3] {
            *channel = (*channel as u32 * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
    image::RgbaImage::from_raw(width, height, data).unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vector_contains_no_font_or_bitmap_dependency_and_stays_inside_canvas() {
        let source = std::str::from_utf8(SVG).unwrap();
        assert!(!source.contains("<text"));
        assert!(!source.contains("<image"));
        assert!(!source.contains("filter="));
        for width in [190, 218, 300, 436, 600, 1200] {
            let height = (width as f32 / aspect()).round() as u32;
            let image = raster(width, height);
            assert!(image.pixels().any(|p| p[3] > 0 && p[3] < 255));
            assert!(image
                .enumerate_pixels()
                .any(|(x, _, p)| x > width * 3 / 4 && p[3] > 200));
            assert!(image.enumerate_pixels().all(|(x, y, p)| (x > 0
                && y > 0
                && x + 1 < width
                && y + 1 < height)
                || p[3] == 0));
        }
    }
}
