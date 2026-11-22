//! Tiny bitmap-font renderer for menu labels.
//!
//! Glyphs are 1-bit rows (`font_data.rs`, generated). A label is rasterized
//! at runtime into a small RGBA image at the 640×360 render scale, so menu
//! text is real, code-built UI (not baked into the background) and stays on
//! the same pixel grid as the art. Glyphs are drawn white with a dark 1-px
//! drop shadow; the UI tints them per state with `ImageNode::color`.

#[derive(Debug)]
pub struct Glyph {
    pub ch: char,
    pub width: u32,
    /// One bit mask per row, bit 0 = leftmost pixel.
    pub rows: &'static [u32],
}

#[derive(Debug)]
pub struct PixelFont {
    pub height: u32,
    pub baseline: u32,
    pub space: u32,
    pub spacing: u32,
    pub glyphs: &'static [Glyph],
}

/// Shadow color baked under the glyphs (tinting darkens it further).
pub const SHADOW: [u8; 4] = [16, 12, 26, 200];

/// An RGBA8 bitmap, row-major, top row first.
#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Bitmap {
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

impl PixelFont {
    pub fn glyph(&self, ch: char) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| g.ch == ch)
    }

    /// Ink width of `text` in pixels (without the shadow column).
    pub fn measure(&self, text: &str) -> u32 {
        let mut w = 0;
        let mut first = true;
        for ch in text.chars() {
            match self.glyph(ch) {
                Some(g) => {
                    if !first {
                        w += self.spacing;
                    }
                    w += g.width;
                }
                None => w += self.space,
            }
            first = false;
        }
        w
    }

    /// Rasterize `text`: white glyphs, shadow one pixel down-right.
    pub fn render(&self, text: &str) -> Bitmap {
        let width = self.measure(text).max(1) + 1;
        let height = self.height + 1;
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        let mut put = |x: u32, y: u32, c: [u8; 4], over: bool| {
            let i = ((y * width + x) * 4) as usize;
            if over || rgba[i + 3] == 0 {
                rgba[i..i + 4].copy_from_slice(&c);
            }
        };
        let mut pen = 0;
        let mut first = true;
        let mut ink = Vec::new();
        for ch in text.chars() {
            match self.glyph(ch) {
                Some(g) => {
                    if !first {
                        pen += self.spacing;
                    }
                    for (y, row) in g.rows.iter().enumerate() {
                        for x in 0..g.width {
                            if row & (1 << x) != 0 {
                                ink.push((pen + x, y as u32));
                            }
                        }
                    }
                    pen += g.width;
                }
                None => pen += self.space,
            }
            first = false;
        }
        for &(x, y) in &ink {
            put(x + 1, y + 1, SHADOW, false);
        }
        for &(x, y) in &ink {
            put(x, y, [255, 255, 255, 255], true);
        }
        Bitmap {
            width,
            height,
            rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::font_data::{SANS_SMALL, SERIF};

    #[test]
    fn menu_labels_have_every_glyph() {
        for text in ["Tek Oyunculu", "Çok Oyunculu", "Ayarlar"] {
            for ch in text.chars().filter(|c| *c != ' ') {
                assert!(SERIF.glyph(ch).is_some(), "missing {ch:?}");
            }
        }
        for text in ["YAKINDA", "Yakında", "Seç", "Onayla", "↑↓", "Enter"] {
            for ch in text.chars().filter(|c| *c != ' ') {
                assert!(SANS_SMALL.glyph(ch).is_some(), "missing {ch:?}");
            }
        }
    }

    #[test]
    fn render_is_crisp_and_sized() {
        let bmp = SERIF.render("Tek");
        assert_eq!(bmp.width, SERIF.measure("Tek") + 1);
        assert_eq!(bmp.height, SERIF.height + 1);
        // Only three alpha levels: empty, shadow, glyph (no anti-aliasing).
        for y in 0..bmp.height {
            for x in 0..bmp.width {
                let a = bmp.pixel(x, y)[3];
                assert!(a == 0 || a == 255 || a == super::SHADOW[3]);
            }
        }
        assert!(bmp.rgba.chunks(4).any(|p| p == [255, 255, 255, 255]));
    }

    #[test]
    fn spaces_advance_without_ink() {
        assert!(SERIF.measure("Tek Oyunculu") > SERIF.measure("TekOyunculu"));
    }
}
