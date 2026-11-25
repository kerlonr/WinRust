//! topbar: the frosted glass of the Delphos style (shares main.rs through `super::*`).
//! What is behind a surface is captured and blurred once (the panels: the screen as they open; the bar: the wallpaper), so
//! there is no live blur to pay for and it looks the same with Windows' transparency effects off (battery saver).
//! Everything is painted as usual on the flat background colour; `compose` then turns that colour into glass and every
//! other pixel into the translucent layer that gives the same look over it.
use super::*;
use windows_sys::Win32::Graphics::{
    Dwm::DwmFlush,
    GdiPlus::{GdipCreateBitmapFromFile, GdipDisposeImage, GdipDrawImageRectI, GdipGetImageHeight, GdipGetImageWidth, GpBitmap, GpImage},
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
fn frost(src: &[u32], w: usize, h: usize, tint: u32, k: f32) -> Vec<u32> {
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
    for p in a.iter_mut() {
        let l = 0.3 * p[0] + 0.59 * p[1] + 0.11 * p[2];
        let hold = 1.0 - 0.45 * (l / 255.0); // light areas are pulled down so light text stays readable
        for (c, t) in p.iter_mut().zip(t) {
            let v = (l + (*c - l) * 1.7).clamp(0.0, 255.0) * hold;
            *c = v + (t - v) * k;
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
    let px = frost(d.pixels(), w as usize, h as usize, pal().bg, strength());
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
    let px = frost(d.pixels(), w as usize, h as usize, pal().bg, strength());
    d.free();
    Some(Backdrop { x, y, w, h, px })
}

/// Paints the bar's picture as glass.
pub(crate) unsafe fn compose_bar(d: &Dib, slab: Slab) {
    let y = GetSystemMetrics(SM_CYSCREEN) - d.h;
    let key = key_of((d.w, d.h, pal().bg, i18n::style::bar_opacity()));
    if BAR_GLASS.with_borrow(|g| g.as_ref().is_none_or(|g| g.0 != key)) {
        let made = wallpaper(0, y, d.w, d.h).map(|b| (key, b));
        BAR_GLASS.set(made);
    }
    BAR_GLASS.with_borrow(|g| compose(d, g.as_ref().map(|g| &g.1), 0, y, slab));
}

/// Turns a picture painted on the flat background colour into a glass slab at screen position (x, y): the background becomes
/// the backdrop, the rest the thinnest layer that still gives its colour. A light rim and sheen on top, smooth round corners.
pub(crate) unsafe fn compose(d: &Dib, back: Option<&Backdrop>, x: i32, y: i32, slab: Slab) {
    let key = swap(pal().bg);
    let (w, h) = (d.w as usize, d.h as usize);
    let (l, t, rr, b) = (slab.r.left as f32, slab.r.top as f32, slab.r.right as f32, slab.r.bottom as f32);
    let sheen_h = ((b - t) * 0.5).max(1.0);
    let px = d.pixels();
    for (j, row) in px.chunks_exact_mut(w).enumerate().take(h) {
        let fy = j as f32 + 0.5;
        let sheen = (255.0 * 0.07 * (1.0 - (fy - t) / sheen_h).clamp(0.0, 1.0)) as u32;
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
            };
            let mut c = over(*p & 0xFF_FFFF, key, lighten(base, sheen));
            // only the pixels at an edge need the distance to it
            if near_row || fx < l + slab.rad || fx > rr - slab.rad {
                let (cx, cy) = (fx.clamp(l + slab.rad, rr - slab.rad), fy.clamp(t + slab.rad, b - slab.rad));
                let dist = slab.rad - ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt(); // inside > 0
                let dist = if dist >= slab.rad { slab.rad.min(fx - l).min(rr - fx).min(fy - t).min(b - fy) } else { dist };
                let cover = (dist + 0.5).clamp(0.0, 1.0);
                let edge = (1.0 - (dist - 0.5).abs()).clamp(0.0, 1.0);
                c = lighten(c, (rim * edge) as u32);
                let a = (cover * 255.0) as u32;
                // never fully clear: the gap around the bar still takes clicks, so the screen's edge stays a target
                *p = if a == 0 { 0x0100_0000 } else { a << 24 | scale(c >> 16 & 255, a) << 16 | scale(c >> 8 & 255, a) << 8 | scale(c & 255, a) };
            } else {
                *p = 0xFF00_0000 | c;
            }
        }
    }
}

fn scale(v: u32, a: u32) -> u32 {
    (v * a + 127) / 255
}

/// White at `a` (0..255) over a colour.
fn lighten(c: u32, a: u32) -> u32 {
    if a == 0 {
        return c;
    }
    let f = |s: u32| {
        let v = c >> s & 255;
        (v + scale(255 - v, a)) << s
    };
    f(16) | f(8) | f(0)
}

/// `p` was painted on `key`: the same paint laid over `base` instead.
fn over(p: u32, key: u32, base: u32) -> u32 {
    if p == key {
        return base;
    }
    // the smallest opacity that explains the change in every channel
    let mut a = 0u32;
    for s in [16, 8, 0] {
        let (v, k) = (p >> s & 255, key >> s & 255);
        let need = if v >= k { ((v - k) * 255).div_ceil((255 - k).max(1)) } else { ((k - v) * 255).div_ceil(k.max(1)) };
        a = a.max(need.min(255));
    }
    let mut out = 0;
    for s in [16, 8, 0] {
        let (v, k, b) = (p >> s & 255, key >> s & 255, base >> s & 255);
        let own = v.saturating_sub(scale(k, 255 - a));
        out |= (own + scale(b, 255 - a)).min(255) << s;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{frost, over, swap};

    #[test]
    fn paint_over_its_own_background_is_unchanged() {
        let key = 0x1B1B1E;
        for p in [0x1B1B1E, 0xFFFFFF, 0x000000, 0x8FD3FF, 0x2A2A2F, 0x101216] {
            let q = over(p, key, key);
            for s in [16, 8, 0] {
                assert!((q >> s & 255).abs_diff(p >> s & 255) <= 2, "{p:06X} -> {q:06X}");
            }
        }
        assert_eq!(over(key, key, 0x123456), 0x123456); // the background itself is clear glass
        assert_eq!(over(0xFFFFFF, key, 0x123456), 0xFFFFFF);
    }

    #[test]
    fn frosting_keeps_the_size_and_a_flat_picture_flat() {
        let out = frost(&vec![0x406080; 37 * 21], 37, 21, swap(0x1B1B1E), 0.5);
        assert_eq!(out.len(), 37 * 21);
        assert!(out.iter().all(|&p| (p >> 8 & 255).abs_diff(out[0] >> 8 & 255) <= 1));
    }
}
