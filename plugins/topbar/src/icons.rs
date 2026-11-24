//! topbar: line icons of the Delphos style, drawn on a 24x24 grid with one thin round-ended stroke (shares main.rs through `super::*`).
use super::*;
use windows_sys::Win32::Graphics::GdiPlus::{GdipDrawEllipse, GdipDrawPath, GdipSetPenLineJoin, LineJoinRound};

pub(crate) const START: &str = "start"; // not a font glyph: the other styles paint the logo's tiles

/// The style draws its own icons instead of the icon font's.
pub(crate) fn line_icons() -> bool {
    style() == Style::Delphos
}

/// Resting colour of the bar's status icons: a step quieter than the text in the Delphos style.
pub(crate) fn icon_ink() -> u32 {
    if line_icons() { mix(pal().dim, pal().text, 0.7) } else { pal().text }
}

/// The volume glyph for a level.
pub(crate) fn vol_glyph(vol: i32, muted: bool) -> &'static str {
    if muted || vol == 0 { "\u{E74F}" } else if vol < 34 { "\u{E993}" } else if vol < 67 { "\u{E994}" } else { "\u{E995}" }
}

/// An icon centered in `r`, as tall as the selected font: the line icon where the style has one, else the font's own glyph.
pub(crate) unsafe fn sym(dc: HDC, s: &str, r: RECT, c: u32) {
    sym_t(dc, s, r, c, 0.0);
}

/// `sym` at hover progress `t` (0..1): the icons that react to the pointer move with it.
pub(crate) unsafe fn sym_t(dc: HDC, s: &str, r: RECT, c: u32, t: f32) {
    if !line_icons() || !draw(dc, s, r, c, t) {
        text(dc, s, r, c, DT_CENTER);
    }
}

/// A pen on the 24x24 grid.
struct Ink {
    g: *mut GpGraphics,
    p: *mut GpPen,
    x: f32,
    y: f32,
    k: f32,
    argb: u32,
    t: f32,
}

impl Ink {
    unsafe fn line(&self, pts: &[(f32, f32)]) {
        for w in pts.windows(2) {
            GdipDrawLine(self.g, self.p, self.x + w[0].0 * self.k, self.y + w[0].1 * self.k, self.x + w[1].0 * self.k, self.y + w[1].1 * self.k);
        }
    }
    unsafe fn circle(&self, cx: f32, cy: f32, r: f32) {
        GdipDrawEllipse(self.g, self.p, self.x + (cx - r) * self.k, self.y + (cy - r) * self.k, 2.0 * r * self.k, 2.0 * r * self.k);
    }
    unsafe fn dot(&self, cx: f32, cy: f32, r: f32) {
        let mut b: *mut GpSolidFill = null_mut();
        GdipCreateSolidFill(self.argb, &mut b);
        GdipFillEllipse(self.g, b as *mut GpBrush, self.x + (cx - r) * self.k, self.y + (cy - r) * self.k, 2.0 * r * self.k, 2.0 * r * self.k);
        GdipDeleteBrush(b as *mut GpBrush);
    }
    unsafe fn arc(&self, cx: f32, cy: f32, r: f32, from: f32, sweep: f32) {
        GdipDrawArc(self.g, self.p, self.x + (cx - r) * self.k, self.y + (cy - r) * self.k, 2.0 * r * self.k, 2.0 * r * self.k, from, sweep);
    }
    /// A rounded rectangle, outlined or filled.
    unsafe fn rect(&self, x: f32, y: f32, w: f32, h: f32, r: f32, solid: bool) {
        let (x, y, w, h) = (self.x + x * self.k, self.y + y * self.k, w * self.k, h * self.k);
        let d = (2.0 * r * self.k).min(w).min(h);
        let mut path: *mut GpPath = null_mut();
        GdipCreatePath(FillModeAlternate, &mut path);
        GdipAddPathArc(path, x, y, d, d, 180.0, 90.0);
        GdipAddPathArc(path, x + w - d, y, d, d, 270.0, 90.0);
        GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0.0, 90.0);
        GdipAddPathArc(path, x, y + h - d, d, d, 90.0, 90.0);
        GdipClosePathFigure(path);
        if solid {
            let mut b: *mut GpSolidFill = null_mut();
            GdipCreateSolidFill(self.argb, &mut b);
            GdipFillPath(self.g, b as *mut GpBrush, path);
            GdipDeleteBrush(b as *mut GpBrush);
        } else {
            GdipDrawPath(self.g, self.p, path);
        }
        GdipDeletePath(path);
    }
}

/// Draws the line icon standing in for the font glyph `s`; false when there is none.
unsafe fn draw(dc: HDC, s: &str, r: RECT, c: u32, t: f32) -> bool {
    let mut lf: LOGFONTW = zeroed();
    GetObjectW(GetCurrentObject(dc, OBJ_FONT as u32), size_of::<LOGFONTW>() as i32, &mut lf as *mut _ as *mut _);
    let px = (lf.lfHeight.abs() as f32 * 1.125).max(10.0); // the 24-unit box keeps a margin the font's glyphs do not have
    let mut g: *mut GpGraphics = null_mut();
    if GdipCreateFromHDC(dc, &mut g) != 0 || g.is_null() {
        return false;
    }
    GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
    GdipSetPixelOffsetMode(g, PixelOffsetModeHalf);
    let k = px / 24.0;
    let argb = 0xFF00_0000 | (c & 255) << 16 | (c & 0xFF00) | (c >> 16 & 255);
    let mut p: *mut GpPen = null_mut();
    GdipCreatePen1(argb, (1.8 * k).max(1.25), UnitPixel, &mut p);
    GdipSetPenStartCap(p, LineCapRound);
    GdipSetPenEndCap(p, LineCapRound);
    GdipSetPenLineJoin(p, LineJoinRound);
    let i = Ink { g, p, x: (r.left + r.right) as f32 / 2.0 - px / 2.0, y: (r.top + r.bottom) as f32 / 2.0 - px / 2.0, k, argb, t };
    let known = shape(&i, s);
    GdipDeletePen(p);
    GdipDeleteGraphics(g);
    known
}

unsafe fn shape(i: &Ink, s: &str) -> bool {
    match s {
        // settings: two sliders
        "\u{E713}" => {
            // the knobs trade places under the pointer
            for (x, y) in [(15.0 - 6.0 * i.t, 8.0), (9.0 + 6.0 * i.t, 16.0)] {
                i.line(&[(4.0, y), (x - 3.0, y)]);
                i.line(&[(x + 3.0, y), (20.0, y)]);
                i.circle(x, y, 2.8);
            }
        }
        // search
        "\u{E721}" => {
            i.circle(11.0, 11.0, 6.5);
            i.line(&[(15.8, 15.8), (20.0, 20.0)]);
        }
        // task view: a window in front of another
        "\u{E7C4}" => {
            i.rect(8.5, 8.5, 12.0, 12.0, 3.0, false);
            i.line(&[(3.5, 13.0), (3.5, 6.5)]);
            i.arc(6.5, 6.5, 3.0, 180.0, 90.0);
            i.line(&[(6.5, 3.5), (13.0, 3.5)]);
        }
        // volume: more waves with the level, a cross when muted
        "\u{E74F}" | "\u{E993}" | "\u{E994}" | "\u{E995}" | "\u{E767}" => {
            i.line(&[(3.5, 9.5), (6.5, 9.5), (11.0, 5.5), (11.0, 18.5), (6.5, 14.5), (3.5, 14.5), (3.5, 9.5)]);
            if s == "\u{E74F}" {
                i.line(&[(15.5, 9.5), (20.5, 14.5)]);
                i.line(&[(20.5, 9.5), (15.5, 14.5)]);
            } else {
                i.arc(11.5, 12.0, 4.0, -42.0, 84.0);
                if s != "\u{E993}" {
                    i.arc(11.5, 12.0, 8.0, -42.0, 84.0);
                }
            }
        }
        // battery: filled to the charge; a bolt while plugged in
        "\u{E83E}" | "\u{E83F}" => {
            let pct = BAT.get().0.clamp(0, 100) as f32 / 100.0;
            i.rect(2.0, 7.0, 17.5, 10.0, 3.0, false);
            i.line(&[(22.2, 10.5), (22.2, 13.5)]);
            if s == "\u{E83E}" {
                i.line(&[(11.8, 9.0), (9.0, 12.3), (12.5, 12.3), (9.8, 15.0)]);
            } else {
                i.rect(4.3, 9.3, (12.9 * pct).max(1.5), 5.4, 1.2, true);
            }
        }
        // wired network: a node over two
        "\u{E839}" => {
            i.rect(8.5, 3.5, 7.0, 6.0, 1.8, false);
            i.rect(3.0, 14.5, 7.0, 6.0, 1.8, false);
            i.rect(14.0, 14.5, 7.0, 6.0, 1.8, false);
            i.line(&[(12.0, 9.5), (12.0, 12.0)]);
            i.line(&[(6.5, 14.5), (6.5, 12.0), (17.5, 12.0), (17.5, 14.5)]);
        }
        "\u{E702}" => i.line(&[(7.0, 7.5), (16.5, 16.5), (12.0, 20.5), (12.0, 3.5), (16.5, 7.5), (7.0, 16.5)]), // Bluetooth
        // brightness
        "\u{E706}" => {
            i.circle(12.0, 12.0, 4.0);
            for n in 0..8 {
                let (y, x) = (n as f32 * std::f32::consts::FRAC_PI_4).sin_cos();
                i.line(&[(12.0 + 7.2 * x, 12.0 + 7.2 * y), (12.0 + 9.0 * x, 12.0 + 9.0 * y)]);
            }
        }
        // airplane mode: a paper plane
        "\u{E709}" => {
            i.line(&[(20.5, 3.5), (3.5, 10.0), (10.5, 13.5), (14.0, 20.5), (20.5, 3.5)]);
            i.line(&[(20.5, 3.5), (10.5, 13.5)]);
        }
        // accessibility
        "\u{E776}" => {
            i.circle(12.0, 12.0, 9.0);
            i.dot(12.0, 7.6, 1.4);
            i.line(&[(7.5, 10.5), (12.0, 11.3), (16.5, 10.5)]);
            i.line(&[(9.5, 17.0), (12.0, 13.2), (14.5, 17.0)]);
            i.line(&[(12.0, 11.3), (12.0, 13.2)]);
        }
        "\u{E945}" => i.line(&[(13.0, 3.0), (5.0, 13.5), (11.5, 13.5), (10.5, 21.0), (19.0, 10.0), (12.5, 10.0), (13.0, 3.0)]), // energy saver
        // captions
        "\u{E7F0}" => {
            i.rect(3.0, 5.0, 18.0, 14.0, 3.5, false);
            i.line(&[(7.0, 11.0), (11.0, 11.0)]);
            i.line(&[(14.0, 11.0), (17.0, 11.0)]);
            i.line(&[(7.0, 15.0), (8.5, 15.0)]);
            i.line(&[(11.5, 15.0), (17.0, 15.0)]);
        }
        "\u{E76C}" => i.line(&[(9.0, 5.5), (15.5, 12.0), (9.0, 18.5)]),  // chevron right
        "\u{E70E}" => i.line(&[(5.5, 15.0), (12.0, 8.5), (18.5, 15.0)]), // chevron up
        "\u{E70D}" => i.line(&[(5.5, 9.0), (12.0, 15.5), (18.5, 9.0)]),  // chevron down
        "\u{E73E}" => i.line(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)]), // check mark
        // Start: four outlined tiles
        START => {
            for (x, y) in [(3.5, 3.5), (13.5, 3.5), (3.5, 13.5), (13.5, 13.5)] {
                i.rect(x, y, 7.0, 7.0, 2.0 + 1.5 * i.t, false); // tiles round into dots under the pointer
            }
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_glyph_follows_the_level() {
        assert_eq!(vol_glyph(80, true), "\u{E74F}");
        assert_eq!(vol_glyph(0, false), "\u{E74F}");
        assert_eq!(vol_glyph(10, false), "\u{E993}");
        assert_eq!(vol_glyph(50, false), "\u{E994}");
        assert_eq!(vol_glyph(100, false), "\u{E995}");
    }
}
