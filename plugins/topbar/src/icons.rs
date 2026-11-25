//! topbar: line icons of the Delphos style, drawn on a 24x24 grid with one thin round-ended stroke (shares main.rs through `super::*`).
use super::*;
use windows_sys::Win32::Graphics::GdiPlus::{
    GdipCreateBitmapFromScan0, GdipCreateLineBrush, GdipDisposeImage, GdipDrawEllipse, GdipDrawImageRect, GdipDrawPath, GdipFillPolygon, GdipSetInterpolationMode, GdipSetPenLineJoin, GpBitmap, GpImage, GpLineGradient, InterpolationModeHighQualityBicubic,
    LineJoinRound, PointF, WrapModeTileFlipXY,
};

thread_local! {
    // app icons as GDI+ bitmaps (with the pixels they point to): drawn at any size without jagged edges
    static SMOOTH: RefCell<HashMap<usize, (*mut GpBitmap, Vec<u32>)>> = RefCell::new(HashMap::new());
}

/// Size app icons are loaded at: twice what they are drawn at in the Delphos style, so hover and click can scale them cleanly.
pub(crate) fn load_px() -> i32 {
    sc(if line_icons() { 48 } else { 24 })
}

/// Keeps a smooth copy of icon `ic`, whose 32-bit colour bitmap is `color` (px x px).
pub(crate) unsafe fn remember(ic: HICON, color: HBITMAP, px: i32) {
    let mut bi: BITMAPINFO = zeroed();
    bi.bmiHeader = BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: px, biHeight: -px, biPlanes: 1, biBitCount: 32, ..zeroed() };
    let mut bits = vec![0u32; (px * px) as usize];
    let dc = GetDC(null_mut());
    let got = GetDIBits(dc, color, 0, px as u32, bits.as_mut_ptr() as *mut _, &mut bi, DIB_RGB_COLORS);
    ReleaseDC(null_mut(), dc);
    forget_icon(ic); // a handle used again for another icon
    if got != px || ic.is_null() || bits.iter().all(|p| p >> 24 == 0) {
        return; // no alpha: DrawIconEx knows its mask
    }
    let straight = bits.iter().any(|p| [16, 8, 0].iter().any(|s| p >> s & 255 > p >> 24));
    let mut b: *mut GpBitmap = null_mut();
    let fmt = if straight { 0x0026_200A } else { 0x000E_200B }; // PixelFormat32bppARGB / 32bppPARGB
    if GdipCreateBitmapFromScan0(px, px, px * 4, fmt, bits.as_ptr() as *const u8, &mut b) == 0 && !b.is_null() {
        SMOOTH.with_borrow_mut(|m| m.insert(ic as usize, (b, bits)));
    }
}

/// `remember` for an icon of any origin (a tray icon).
pub(crate) unsafe fn remember_icon(ic: HICON) {
    let mut ii: ICONINFO = zeroed();
    if GetIconInfo(ic, &mut ii) == 0 {
        return;
    }
    let mut b: BITMAP = zeroed();
    if !ii.hbmColor.is_null() && GetObjectW(ii.hbmColor, size_of::<BITMAP>() as i32, &mut b as *mut _ as *mut _) != 0 && b.bmWidth == b.bmHeight && (8..=256).contains(&b.bmWidth) {
        remember(ic, ii.hbmColor, b.bmWidth);
    }
    DeleteObject(ii.hbmColor);
    DeleteObject(ii.hbmMask);
}

/// Drops the smooth copy of an icon about to be destroyed.
pub(crate) unsafe fn forget_icon(ic: HICON) {
    if let Some((b, _)) = SMOOTH.with_borrow_mut(|m| m.remove(&(ic as usize))) {
        GdipDisposeImage(b as *mut GpImage);
    }
}

/// An app icon centered on (cx, cy), `size` px wide.
pub(crate) unsafe fn app_icon(dc: HDC, ic: HICON, cx: f32, cy: f32, size: f32) {
    let mut g: *mut GpGraphics = null_mut();
    if let Some(b) = SMOOTH.with_borrow(|m| m.get(&(ic as usize)).map(|e| e.0)) {
        if GdipCreateFromHDC(dc, &mut g) == 0 && !g.is_null() {
            GdipSetInterpolationMode(g, InterpolationModeHighQualityBicubic);
            GdipSetPixelOffsetMode(g, PixelOffsetModeHalf);
            GdipDrawImageRect(g, b as *mut GpImage, cx - size / 2.0, cy - size / 2.0, size, size);
            GdipDeleteGraphics(g);
            return;
        }
    }
    let s = size as i32;
    DrawIconEx(dc, cx as i32 - s / 2, cy as i32 - s / 2, ic, s, s, 0, null_mut(), DI_NORMAL);
}

/// The Windows logo: four solid panes in one blue gradient, on whole pixels; a little brighter under the pointer (`t`),
/// giving way to a click (`p`).
pub(crate) unsafe fn win_logo(dc: HDC, r: RECT, size: i32, t: f32, p: f32) {
    let mut g: *mut GpGraphics = null_mut();
    if GdipCreateFromHDC(dc, &mut g) != 0 || g.is_null() {
        return;
    }
    GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
    GdipSetPixelOffsetMode(g, PixelOffsetModeHalf);
    let want = size as f32 * (1.0 - 0.08 * p);
    let gap = (want * 0.055).round().max(1.0);
    let pane = ((want - gap) / 2.0).round();
    let full = 2.0 * pane + gap;
    let (x, y) = (((r.left + r.right) as f32 / 2.0 - full / 2.0).round(), ((r.top + r.bottom) as f32 / 2.0 - full / 2.0).round());
    let argb = |c: u32| 0xFF00_0000 | (c & 255) << 16 | (c & 0xFF00) | (c >> 16 & 255);
    let (a, b) = (mix(rgb(120, 220, 255), rgb(255, 255, 255), 0.25 * t), mix(rgb(16, 126, 232), rgb(120, 200, 255), 0.35 * t));
    let mut br: *mut GpLineGradient = null_mut();
    GdipCreateLineBrush(&PointF { X: x - 1.0, Y: y - 1.0 }, &PointF { X: x + full + 1.0, Y: y + full + 1.0 }, argb(a), argb(b), WrapModeTileFlipXY, &mut br);
    let d = (pane * 0.16).max(1.0); // the panes' corners are barely rounded
    for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let (px, py) = (x + dx * (pane + gap), y + dy * (pane + gap));
        let mut path: *mut GpPath = null_mut();
        GdipCreatePath(FillModeAlternate, &mut path);
        GdipAddPathArc(path, px, py, d, d, 180.0, 90.0);
        GdipAddPathArc(path, px + pane - d, py, d, d, 270.0, 90.0);
        GdipAddPathArc(path, px + pane - d, py + pane - d, d, d, 0.0, 90.0);
        GdipAddPathArc(path, px, py + pane - d, d, d, 90.0, 90.0);
        GdipClosePathFigure(path);
        GdipFillPath(g, br as *mut GpBrush, path);
        GdipDeletePath(path);
    }
    GdipDeleteBrush(br as *mut GpBrush);
    GdipDeleteGraphics(g);
}

pub(crate) const CONTROL: &str = "control"; // not a font glyph: only the Delphos bar has it

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
    sym_t(dc, s, r, c, 0.0, 0.0);
}

/// `sym` at hover progress `t` (0..1; some icons move a part of themselves) and click progress `p` (gives way).
pub(crate) unsafe fn sym_t(dc: HDC, s: &str, r: RECT, c: u32, t: f32, p: f32) {
    if !line_icons() || !draw(dc, s, r, c, t, p) {
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
    /// A filled shape; the pen rounds its corners.
    unsafe fn poly(&self, pts: &[(f32, f32)]) {
        let v: Vec<PointF> = pts.iter().map(|p| PointF { X: self.x + p.0 * self.k, Y: self.y + p.1 * self.k }).collect();
        let mut b: *mut GpSolidFill = null_mut();
        GdipCreateSolidFill(self.argb, &mut b);
        GdipFillPolygon(self.g, b as *mut GpBrush, v.as_ptr(), v.len() as i32, FillModeAlternate);
        GdipDeleteBrush(b as *mut GpBrush);
        self.line(pts);
        self.line(&[pts[pts.len() - 1], pts[0]]);
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
unsafe fn draw(dc: HDC, s: &str, r: RECT, c: u32, t: f32, press: f32) -> bool {
    let mut lf: LOGFONTW = zeroed();
    GetObjectW(GetCurrentObject(dc, OBJ_FONT as u32), size_of::<LOGFONTW>() as i32, &mut lf as *mut _ as *mut _);
    let px = (lf.lfHeight.abs() as f32 * 1.125).max(10.0) * (1.0 - 0.08 * press); // the 24-unit box keeps a margin the font's glyphs do not have
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
        // control center: two switches, one on and one off
        CONTROL => {
            i.rect(3.0, 4.2, 18.0, 6.8, 3.4, false);
            i.dot(6.6, 7.6, 1.7);
            i.rect(3.0, 13.0, 18.0, 6.8, 3.4, false);
            i.dot(17.4, 16.4, 1.7);
        }
        // folder
        "\u{E8B7}" => i.line(&[(3.5, 18.5), (3.5, 6.0), (9.5, 6.0), (11.8, 8.6), (20.5, 8.6), (20.5, 18.5), (3.5, 18.5)]),
        // power
        "\u{E7E8}" => {
            i.arc(12.0, 12.8, 7.6, -52.0, 284.0);
            i.line(&[(12.0, 3.4), (12.0, 11.0)]);
        }
        // lock
        "\u{E72E}" => {
            i.rect(5.0, 10.5, 14.0, 9.5, 2.6, false);
            i.line(&[(8.2, 10.5), (8.2, 8.0)]);
            i.arc(12.0, 8.0, 3.8, 180.0, 180.0);
            i.line(&[(15.8, 8.0), (15.8, 10.5)]);
        }
        // sleep: a moon
        "\u{E708}" => {
            i.arc(11.6, 12.4, 8.0, 3.0, 264.0);
            i.arc(17.4, 7.0, 6.6, 92.0, 78.0);
            i.line(&[(12.0, 4.4), (10.9, 8.2)]);
            i.line(&[(17.2, 13.6), (19.6, 12.8)]);
        }
        // restart
        "\u{E777}" => {
            i.arc(12.0, 12.0, 7.6, -20.0, 300.0);
            i.line(&[(19.6, 4.6), (19.3, 9.4), (14.6, 9.0)]);
        }
        // web
        "\u{E774}" => {
            i.circle(12.0, 12.0, 8.6);
            i.line(&[(3.4, 12.0), (20.6, 12.0)]);
            i.arc(5.6, 12.0, 10.7, -53.0, 106.0);
            i.arc(18.4, 12.0, 10.7, 127.0, 106.0);
        }
        // calculator
        "\u{E8EF}" => {
            i.rect(5.0, 3.2, 14.0, 17.6, 3.0, false);
            i.line(&[(8.4, 7.4), (15.6, 7.4)]);
            for (x, y) in [(8.8, 12.0), (12.0, 12.0), (15.2, 12.0), (8.8, 16.2), (12.0, 16.2), (15.2, 16.2)] {
                i.dot(x, y, 1.1);
            }
        }
        // document
        "\u{E8A5}" => {
            i.line(&[(13.5, 3.4), (6.0, 3.4), (6.0, 20.6), (18.0, 20.6), (18.0, 8.0), (13.5, 3.4), (13.5, 8.0), (18.0, 8.0)]);
        }
        // keyboard
        "\u{E765}" => {
            i.rect(2.8, 6.0, 18.4, 12.0, 3.0, false);
            for (x, y) in [(6.8, 10.0), (10.3, 10.0), (13.8, 10.0), (17.2, 10.0)] {
                i.dot(x, y, 0.95);
            }
            i.line(&[(8.6, 14.2), (15.4, 14.2)]);
        }
        // pin
        "\u{E718}" => {
            i.line(&[(8.5, 4.0), (15.5, 4.0)]);
            i.line(&[(10.0, 4.0), (10.0, 9.5), (6.5, 14.0), (17.5, 14.0), (14.0, 9.5), (14.0, 4.0)]);
            i.line(&[(12.0, 14.0), (12.0, 20.5)]);
        }
        // cross
        "\u{E711}" => {
            i.line(&[(6.5, 6.5), (17.5, 17.5)]);
            i.line(&[(17.5, 6.5), (6.5, 17.5)]);
        }
        "\u{E76B}" => i.line(&[(15.0, 5.5), (8.5, 12.0), (15.0, 18.5)]), // chevron left
        "\u{E76C}" => i.line(&[(9.0, 5.5), (15.5, 12.0), (9.0, 18.5)]),  // chevron right
        "\u{E70E}" => i.line(&[(5.5, 15.0), (12.0, 8.5), (18.5, 15.0)]), // chevron up
        "\u{E70D}" => i.line(&[(5.5, 9.0), (12.0, 15.5), (18.5, 9.0)]),  // chevron down
        // previous, next track, play, pause
        "\u{E892}" => {
            i.poly(&[(19.0, 5.5), (19.0, 18.5), (8.5, 12.0)]);
            i.line(&[(5.5, 5.5), (5.5, 18.5)]);
        }
        "\u{E893}" => {
            i.poly(&[(5.0, 5.5), (5.0, 18.5), (15.5, 12.0)]);
            i.line(&[(18.5, 5.5), (18.5, 18.5)]);
        }
        "\u{E768}" => i.poly(&[(7.0, 4.5), (7.0, 19.5), (19.5, 12.0)]),
        "\u{E769}" => {
            i.rect(6.0, 5.0, 4.0, 14.0, 1.2, true);
            i.rect(14.0, 5.0, 4.0, 14.0, 1.2, true);
        }
        "\u{E73E}" => i.line(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)]), // check mark
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
