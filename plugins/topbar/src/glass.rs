//! topbar: the frosted glass of the Delphos style (shares main.rs through `super::*`).
//! What is behind a surface is captured and blurred once (the panels: the screen as they open; the bar: the wallpaper), so
//! there is no live blur to pay for and it looks the same with Windows' transparency effects off (battery saver).
//! Everything is painted as usual on the flat background colour; `compose` then turns that colour into glass and every
//! other pixel into the translucent layer that gives the same look over it.
use super::*;
use windows_sys::Win32::Graphics::{
    Dwm::DwmFlush,
    GdiPlus::{GdipCreateBitmapFromFile, GdipCreateBitmapFromScan0, GdipDisposeImage, GdipDrawImageRectI, GdipGetImageGraphicsContext, GdipGetImageHeight, GdipGetImageWidth, GpBitmap, GpImage},
};

/// A blurred picture of what is behind, at screen position (x, y). Pixels are 0x00RRGGBB.
pub(crate) struct Backdrop {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    px: Vec<u32>,
}

/// The rounded slab inside a window: its rectangle and corner radius.
#[derive(Clone, Copy)]
pub(crate) struct Slab {
    pub r: RECT,
    pub rad: f32,
}

/// An off-screen 32-bit bitmap whose pixels can be read and written.
#[derive(Clone, Copy)]
pub(crate) struct Dib {
    pub dc: HDC,
    bmp: HBITMAP,
    bits: *mut u32,
    pub w: i32,
    pub h: i32,
}

impl Dib {
    pub(crate) unsafe fn new(w: i32, h: i32) -> Option<Dib> {
        let (w, h) = (w.max(1), h.max(1));
        let mut bi: BITMAPINFO = zeroed();
        bi.bmiHeader = BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..zeroed() };
        let mut bits: *mut core::ffi::c_void = null_mut();
        let dc = CreateCompatibleDC(null_mut());
        let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bmp.is_null() || bits.is_null() {
            DeleteDC(dc);
            return None;
        }
        SelectObject(dc, bmp);
        Some(Dib { dc, bmp, bits: bits as *mut u32, w, h })
    }
    unsafe fn pixels(&self) -> &mut [u32] {
        GdiFlush();
        std::slice::from_raw_parts_mut(self.bits, (self.w * self.h) as usize)
    }
    /// Makes the pixels of `r` opaque again: GDI+ leaves their alpha undefined.
    pub(crate) unsafe fn solid(&self, r: RECT) {
        let w = self.w;
        let (l, t, rr, b) = (r.left.clamp(0, w), r.top.clamp(0, self.h), r.right.clamp(0, w), r.bottom.clamp(0, self.h));
        let px = self.pixels();
        for y in t..b {
            px[(y * w + l) as usize..(y * w + rr.max(l)) as usize].iter_mut().for_each(|p| *p |= 0xFF00_0000);
        }
    }
    /// An app icon with its own alpha (GDI would leave it on an opaque square). False: the icon has no smooth copy.
    pub(crate) unsafe fn icon(&self, ic: HICON, cx: f32, cy: f32, size: f32) -> bool {
        GdiFlush();
        let (mut img, mut g): (*mut GpBitmap, *mut GpGraphics) = (null_mut(), null_mut());
        // 0x000E_200B: PixelFormat32bppPARGB
        if GdipCreateBitmapFromScan0(self.w, self.h, self.w * 4, 0x000E_200B, self.bits as *const u8, &mut img) != 0 || img.is_null() {
            return false;
        }
        let mut done = false;
        if GdipGetImageGraphicsContext(img as *mut GpImage, &mut g) == 0 && !g.is_null() {
            done = smooth_icon(g, ic, cx, cy, size);
            GdipDeleteGraphics(g);
        }
        GdipDisposeImage(img as *mut GpImage);
        done
    }
    pub(crate) unsafe fn free(self) {
        DeleteDC(self.dc);
        DeleteObject(self.bmp);
    }
}

thread_local! {
    static LAYERED: RefCell<std::collections::HashSet<isize>> = RefCell::new(Default::default()); // windows drawn by UpdateLayeredWindow
    static BAR_GLASS: RefCell<Option<(u64, Backdrop)>> = const { RefCell::new(None) };
}

/// Switches a layered window between constant opacity (SetLayeredWindowAttributes) and per-pixel alpha: Windows accepts the
/// other kind only after the layered bit was taken off and put back.
pub(crate) unsafe fn per_pixel(h: HWND, on: bool) {
    let changed = LAYERED.with_borrow_mut(|s| if on { s.insert(h as isize) } else { s.remove(&(h as isize)) });
    if changed {
        let ex = GetWindowLongW(h, GWL_EXSTYLE);
        SetWindowLongW(h, GWL_EXSTYLE, ex & !(WS_EX_LAYERED as i32));
        SetWindowLongW(h, GWL_EXSTYLE, ex | WS_EX_LAYERED as i32);
    }
}

pub(crate) fn forget(h: HWND) {
    LAYERED.with_borrow_mut(|s| s.remove(&(h as isize)));
}

/// Shows the picture in the window, `alpha` over all of it.
pub(crate) unsafe fn present(h: HWND, d: &Dib, alpha: u8) {
    let (size, src) = (SIZE { cx: d.w, cy: d.h }, POINT { x: 0, y: 0 });
    let b = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: 1 };
    UpdateLayeredWindow(h, null_mut(), null(), &size, d.dc, &src, 0, &b, ULW_ALPHA);
}

/// Moves / fades a window shown by `present` without drawing it again.
pub(crate) unsafe fn fade(h: HWND, x: i32, y: i32, alpha: u8) {
    let pos = POINT { x, y };
    let b = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: 1 };
    UpdateLayeredWindow(h, null_mut(), &pos, null(), null_mut(), null(), 0, &b, ULW_ALPHA);
}

/// COLORREF <-> 0x00RRGGBB (its own inverse).
fn swap(c: u32) -> u32 {
    (c & 255) << 16 | (c & 0xFF00) | (c >> 16 & 255)
}

/// Blurs a picture, livens its colours, holds bright areas down and lays the tint (COLORREF) over it at strength `k`.
fn frost(src: &[u32], w: usize, h: usize, tint: u32, k: f32, aurora: f32) -> Vec<u32> {
    const D: usize = 8;
    let (sw, sh) = (w.div_ceil(D), h.div_ceil(D));
    let mut a: Vec<[f32; 3]> = Vec::with_capacity(sw * sh);
    for y in 0..sh {
        for x in 0..sw {
            let mut s = [0u32; 3];
            let mut n = 0;
            for yy in (y * D..(y * D + D).min(h)).step_by(3) {
                for xx in (x * D..(x * D + D).min(w)).step_by(3) {
                    let p = src[yy * w + xx];
                    s[0] += p >> 16 & 255;
                    s[1] += p >> 8 & 255;
                    s[2] += p & 255;
                    n += 1;
                }
            }
            a.push(s.map(|v| v as f32 / n.max(1) as f32));
        }
    }
    // three box passes are a gaussian to the eye
    let mut b = a.clone();
    for _ in 0..3 {
        blur_lines(&a, &mut b, sw, sh, 1, sw, 4);
        blur_lines(&b, &mut a, sh, sw, sw, 1, 4);
    }
    let t = swap(tint);
    let t = [(t >> 16 & 255) as f32, (t >> 8 & 255) as f32, (t & 255) as f32];
    // the aurora of the Delphos app: a violet light at the upper left, an ice one at the lower right
    let glow = |x: f32, y: f32, cx: f32, cy: f32| (-((x - cx) / 0.42).powi(2) - ((y - cy) / 0.75).powi(2)).exp();
    for (n, p) in a.iter_mut().enumerate() {
        let (x, y) = ((n % sw) as f32 / sw as f32, (n / sw) as f32 / sh as f32);
        let (violet, ice) = (0.20 * aurora * glow(x, y, 0.12, 0.25), 0.15 * aurora * glow(x, y, 0.9, 0.85));
        let l = 0.3 * p[0] + 0.59 * p[1] + 0.11 * p[2];
        let hold = 1.0 - 0.45 * (l / 255.0); // light areas are pulled down so light text stays readable
        for (c, t) in p.iter_mut().zip(t) {
            let v = (l + (*c - l) * 1.7).clamp(0.0, 255.0) * hold;
            *c = v + (t - v) * k;
        }
        for (c, (v, i)) in p.iter_mut().zip([(157.0, 143.0), (140.0, 211.0), (255.0, 255.0)]) {
            *c += (v - *c) * violet;
            *c += (i - *c) * ice;
        }
    }
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        let fy = ((y as f32 + 0.5) / D as f32 - 0.5).max(0.0);
        let (y0, v) = ((fy as usize).min(sh - 1), fy.fract());
        let y1 = (y0 + 1).min(sh - 1);
        for x in 0..w {
            let fx = ((x as f32 + 0.5) / D as f32 - 0.5).max(0.0);
            let (x0, u) = ((fx as usize).min(sw - 1), fx.fract());
            let x1 = (x0 + 1).min(sw - 1);
            let (p, q, r, s) = (a[y0 * sw + x0], a[y0 * sw + x1], a[y1 * sw + x0], a[y1 * sw + x1]);
            let c = |i: usize| {
                let (m, n) = (p[i] + (q[i] - p[i]) * u, r[i] + (s[i] - r[i]) * u);
                (m + (n - m) * v + 0.5) as u32
            };
            out.push(c(0) << 16 | c(1) << 8 | c(2));
        }
    }
    out
}

/// One box blur of radius `r` along `lines` lines of `n` values (`step` apart, the lines `jump` apart).
fn blur_lines(src: &[[f32; 3]], dst: &mut [[f32; 3]], n: usize, lines: usize, step: usize, jump: usize, r: i32) {
    let d = (2 * r + 1) as f32;
    for l in 0..lines {
        let at = |i: i32| src[l * jump + i.clamp(0, n as i32 - 1) as usize * step];
        let mut s = [0f32; 3];
        for i in -r..=r {
            let p = at(i);
            (0..3).for_each(|c| s[c] += p[c]);
        }
        for i in 0..n as i32 {
            dst[l * jump + i as usize * step] = s.map(|v| v / d);
            let (p, q) = (at(i + r + 1), at(i - r));
            (0..3).for_each(|c| s[c] += p[c] - q[c]);
        }
    }
}

/// How strongly the tint covers the glass; the bar's transparency setting (30..100 %) moves it.
fn strength() -> f32 {
    0.20 + 0.36 * (i18n::style::bar_opacity() as f32 / 100.0)
}

/// The screen in `r` as it looks now, frosted. `flush`: a panel closed just now leaves the screen first.
pub(crate) unsafe fn capture(r: RECT, flush: bool) -> Option<Backdrop> {
    if flush {
        DwmFlush();
    }
    let (x, y) = (r.left, r.top.max(0));
    let (w, h) = (r.right - x, r.bottom.min(GetSystemMetrics(SM_CYSCREEN)) - y);
    let d = Dib::new(w, h)?;
    let (w, h) = (d.w, d.h);
    let scr = GetDC(null_mut());
    BitBlt(d.dc, 0, 0, w, h, scr, x, y, SRCCOPY);
    ReleaseDC(null_mut(), scr);
    let px = frost(d.pixels(), w as usize, h as usize, pal().bg, strength(), 1.0);
    d.free();
    Some(Backdrop { x, y, w, h, px })
}

/// The wallpaper changed: the bar's glass is made again.
pub(crate) fn wallpaper_changed() {
    BAR_GLASS.set(None);
}

/// The wallpaper under the bar (it fills the screen, centered; a plain desktop gives its colour), frosted.
unsafe fn wallpaper(x: i32, y: i32, w: i32, h: i32) -> Option<Backdrop> {
    let d = Dib::new(w, h)?;
    let b = CreateSolidBrush(GetSysColor(COLOR_BACKGROUND));
    FillRect(d.dc, &RECT { left: 0, top: 0, right: w, bottom: h }, b);
    DeleteObject(b);
    let mut p = [0u16; 520];
    SystemParametersInfoW(SPI_GETDESKWALLPAPER, 520, p.as_mut_ptr() as *mut _, 0);
    let (mut img, mut g): (*mut GpBitmap, *mut GpGraphics) = (null_mut(), null_mut());
    if p[0] != 0 && GdipCreateBitmapFromFile(p.as_ptr(), &mut img) == 0 && !img.is_null() {
        let (mut iw, mut ih) = (0u32, 0u32);
        GdipGetImageWidth(img as *mut GpImage, &mut iw);
        GdipGetImageHeight(img as *mut GpImage, &mut ih);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN) as f32, GetSystemMetrics(SM_CYSCREEN) as f32);
        let k = (sw / iw.max(1) as f32).max(sh / ih.max(1) as f32);
        let (dw, dh) = (iw as f32 * k, ih as f32 * k);
        if GdipCreateFromHDC(d.dc, &mut g) == 0 {
            GdipDrawImageRectI(g, img as *mut GpImage, ((sw - dw) / 2.0) as i32 - x, ((sh - dh) / 2.0) as i32 - y, dw as i32, dh as i32);
            GdipDeleteGraphics(g);
        }
        GdipDisposeImage(img as *mut GpImage);
    }
    let px = frost(d.pixels(), w as usize, h as usize, pal().bg, strength(), 0.25); // the wallpaper has its own colour
    d.free();
    Some(Backdrop { x, y, w, h, px })
}

/// The bar's transparency setting as the alpha of its glass: 100 % = solid glass, 30 % (the lowest) = only the wallpaper.
fn glass_alpha(opacity: u8) -> u32 {
    (opacity as u32).saturating_sub(30).min(70) * 255 / 70
}

/// Paints the bar's picture as glass.
pub(crate) unsafe fn compose_bar(d: &Dib, slab: Slab) {
    let y = GetSystemMetrics(SM_CYSCREEN) - d.h;
    let key = key_of((d.w, d.h, pal().bg, i18n::style::bar_opacity()));
    if BAR_GLASS.with_borrow(|g| g.as_ref().is_none_or(|g| g.0 != key)) {
        let made = wallpaper(0, y, d.w, d.h).map(|b| (key, b));
        BAR_GLASS.set(made);
    }
    BAR_GLASS.with_borrow(|g| compose(d, g.as_ref().map(|g| &g.1), 0, y, slab, glass_alpha(i18n::style::bar_opacity())));
}

/// Turns a picture painted on the flat background colour into a glass slab at screen position (x, y): the background becomes
/// the backdrop, the rest the thinnest layer that still gives its colour. A light rim and sheen on top, smooth round corners.
/// `glass` (0..=255): how opaque the backdrop is; what is painted on it stays as it is.
pub(crate) unsafe fn compose(d: &Dib, back: Option<&Backdrop>, x: i32, y: i32, slab: Slab, glass: u32) {
    let key = swap(pal().bg);
    let (w, h) = (d.w as usize, d.h as usize);
    let (l, t, rr, b) = (slab.r.left as f32, slab.r.top as f32, slab.r.right as f32, slab.r.bottom as f32);
    let sheen_h = ((b - t) * 0.5).max(1.0);
    let px = d.pixels();
    for (j, row) in px.chunks_exact_mut(w).enumerate().take(h) {
        let fy = j as f32 + 0.5;
        let sheen = (255.0 * 0.05 * (1.0 - (fy - t) / sheen_h).clamp(0.0, 1.0)) as u32;
        let rim = 255.0 * (0.30 - 0.20 * ((fy - t) / (b - t)).clamp(0.0, 1.0));
        let near_row = fy < t + slab.rad || fy > b - slab.rad;
        let line = back.and_then(|g| {
            let gy = (y + j as i32 - g.y).clamp(0, g.h - 1);
            g.px.get((gy * g.w) as usize..((gy + 1) * g.w) as usize).map(|s| (s, x - g.x))
        });
        for (i, p) in row.iter_mut().enumerate() {
            let fx = i as f32 + 0.5;
            let base = match line {
                Some((s, off)) => s[(i as i32 + off).clamp(0, s.len() as i32 - 1) as usize],
                None => key,
            } | 0xFF00_0000;
            let mut c = laid(layer(*p & 0xFF_FFFF, key), lighten(base, sheen), glass);
            // only the pixels at an edge need the distance to it
            if near_row || fx < l + slab.rad || fx > rr - slab.rad {
                let (cx, cy) = (fx.clamp(l + slab.rad, rr - slab.rad), fy.clamp(t + slab.rad, b - slab.rad));
                let dist = slab.rad - ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt(); // inside > 0
                let dist = if dist >= slab.rad { slab.rad.min(fx - l).min(rr - fx).min(fy - t).min(b - fy) } else { dist };
                let cover = (dist + 0.5).clamp(0.0, 1.0);
                let edge = (1.0 - (dist - 0.5).abs()).clamp(0.0, 1.0);
                c = lighten(c, scale((rim * edge) as u32, glass));
                let a = (cover * 255.0) as u32;
                c = scale(c >> 24, a) << 24 | scale(c >> 16 & 255, a) << 16 | scale(c >> 8 & 255, a) << 8 | scale(c & 255, a);
            }
            // never fully clear: the empty parts and the gap around the bar still take clicks, so the screen's edge stays a target
            *p = if c >> 24 == 0 { 0x0100_0000 } else { c };
        }
    }
}

fn scale(v: u32, a: u32) -> u32 {
    (v * a + 127) / 255
}

/// White at `a` (0..255) over a colour with premultiplied alpha, which it keeps.
fn lighten(c: u32, a: u32) -> u32 {
    let top = c >> 24;
    let f = |s: u32| {
        let v = c >> s & 255;
        (v + scale(top.saturating_sub(v), a)) << s
    };
    top << 24 | f(16) | f(8) | f(0)
}

/// `p` was painted on `key`: the paint alone, premultiplied.
fn layer(p: u32, key: u32) -> u32 {
    // the smallest opacity that explains the change in every channel
    let mut a = 0u32;
    for s in [16, 8, 0] {
        let (v, k) = (p >> s & 255, key >> s & 255);
        let need = if v >= k { ((v - k) * 255).div_ceil((255 - k).max(1)) } else { ((k - v) * 255).div_ceil(k.max(1)) };
        a = a.max(need.min(255));
    }
    let mut out = a << 24;
    for s in [16, 8, 0] {
        out |= (p >> s & 255).saturating_sub(scale(key >> s & 255, 255 - a)).min(a) << s;
    }
    out
}

/// A premultiplied layer over the colour `base` shown at opacity `glass`, premultiplied.
fn laid(l: u32, base: u32, glass: u32) -> u32 {
    let k = scale(glass, 255 - (l >> 24));
    let a = (l >> 24) + k;
    let mut out = a << 24;
    for s in [16, 8, 0] {
        out |= ((l >> s & 255) + scale(base >> s & 255, k)).min(a) << s;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{frost, glass_alpha, laid, layer, swap};

    #[test]
    fn paint_over_its_own_background_is_unchanged() {
        let key = 0x1B1B1E;
        for p in [0x1B1B1E, 0xFFFFFF, 0x000000, 0x8FD3FF, 0x2A2A2F, 0x101216] {
            let q = laid(layer(p, key), key, 255);
            assert_eq!(q >> 24, 255);
            for s in [16, 8, 0] {
                assert!((q >> s & 255).abs_diff(p >> s & 255) <= 2, "{p:06X} -> {q:06X}");
            }
        }
        assert_eq!(laid(layer(key, key), 0x123456, 255), 0xFF12_3456); // the background itself is clear glass
        assert_eq!(laid(layer(0xFFFFFF, key), 0x123456, 255), 0xFFFF_FFFF);
    }

    #[test]
    fn the_lowest_opacity_leaves_only_the_paint() {
        let key = 0x1B1B1E;
        assert_eq!((glass_alpha(100), glass_alpha(30), glass_alpha(0)), (255, 0, 0));
        assert_eq!(laid(layer(key, key), 0x123456, glass_alpha(30)), 0);
        assert_eq!(laid(layer(0xFFFFFF, key), 0x123456, glass_alpha(30)), 0xFFFF_FFFF);
        // premultiplied: no channel above the alpha
        for p in [0x8FD3FF, 0x2A2A2F, 0x101216, 0x1B1B1F] {
            for glass in [0, 90, 255] {
                let q = laid(layer(p, key), 0xF0F0F0, glass);
                assert!([16, 8, 0].iter().all(|s| q >> s & 255 <= q >> 24), "{p:06X} -> {q:08X}");
            }
        }
    }

    #[test]
    fn frosting_keeps_the_size_and_is_smooth() {
        let out = frost(&vec![0x406080; 37 * 21], 37, 21, swap(0x1B1B1E), 0.5, 1.0);
        assert_eq!(out.len(), 37 * 21);
        // no noise: neighbours differ by a step of the gradient at most
        assert!(out.windows(2).enumerate().filter(|(i, _)| (i + 1) % 37 != 0).all(|(_, w)| (w[0] >> 8 & 255).abs_diff(w[1] >> 8 & 255) <= 4));
    }
}
