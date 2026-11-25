//! topbar: status (split out of main.rs; shares its items through `super::*`).
use super::*;

pub(crate) unsafe fn endpoint() -> Option<IAudioEndpointVolume> {
    EP.with_borrow_mut(|e| {
        if e.is_none() {
            *e = (|| -> windows::core::Result<IAudioEndpointVolume> {
                let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                en.GetDefaultAudioEndpoint(eRender, eConsole)?.Activate(CLSCTX_ALL, None)
            })()
            .ok();
        }
        e.clone()
    })
}

pub(crate) unsafe fn read_volume() {
    let v = endpoint().and_then(|e| Some(((e.GetMasterVolumeLevelScalar().ok()? * 100.0).round() as i32, e.GetMute().ok()?.as_bool())));
    if v.is_none() {
        EP.set(None); // device changed: look the endpoint up again next time
    }
    VOL.set(v.unwrap_or((-1, false)));
}

pub(crate) unsafe fn set_volume(v: i32) {
    if let Some(e) = endpoint() {
        let _ = e.SetMasterVolumeLevelScalar(v.clamp(0, 100) as f32 / 100.0, null());
        let _ = e.SetMute(false, null());
        read_volume();
    }
}

pub(crate) unsafe fn add_volume(delta: i32) {
    if let Some(e) = endpoint() {
        let v = (VOL.get().0.max(0) + delta).clamp(0, 100);
        let _ = e.SetMasterVolumeLevelScalar(v as f32 / 100.0, null());
        let _ = e.SetMute(false, null());
        read_volume();
    }
}

pub(crate) unsafe fn toggle_mute() {
    if let Some(e) = endpoint() {
        let _ = e.SetMute(!VOL.get().1, null());
        read_volume();
    }
}

thread_local! {
    static WLAN: Cell<usize> = const { Cell::new(0) }; // WLAN client handle, opened once (opening it talks to wlansvc every time)
}

unsafe extern "system" fn wlan_event(d: *mut L2_NOTIFICATION_DATA, _: *mut std::ffi::c_void) {
    // 9 connection start, 10 complete, 11 failed, 20 disconnecting, 21 disconnected; not the scans
    if !d.is_null() && matches!((*d).NotificationCode, 9..=11 | 20 | 21) {
        net_changed();
    }
}

unsafe extern "system" fn hint_event(_: *const std::ffi::c_void, _: windows_sys::Win32::Networking::WinSock::NL_NETWORK_CONNECTIVITY_HINT) {
    net_changed();
}

/// Any thread: the bar looks at the network again.
unsafe fn net_changed() {
    let bar = BAR.load(Relaxed) as HWND;
    if !bar.is_null() {
        PostMessageW(bar, NET_CHANGED, 0, 0);
    }
}

/// Windows tells us when the internet comes and goes (the poll alone would notice up to 3 s late).
pub(crate) unsafe fn watch_net() {
    let mut h: HANDLE = null_mut();
    windows_sys::Win32::NetworkManagement::IpHelper::NotifyNetworkConnectivityHintChange(Some(hint_event), null(), false, &mut h);
}

pub(crate) unsafe fn read_wifi() {
    let mut h: HANDLE = WLAN.get() as HANDLE;
    let (mut q, mut joining, mut ssid) = (-1, false, String::new());
    if h.is_null() {
        let mut ver = 0u32;
        if WlanOpenHandle(2, null(), &mut ver, &mut h) != 0 {
            h = null_mut();
        } else {
            // 8 = ACM notifications: connecting, connected, disconnected
            WlanRegisterNotification(h, 8, 1, Some(wlan_event), null(), null(), null_mut());
        }
        WLAN.set(h as usize);
    }
    if !h.is_null() {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = null_mut();
        if WlanEnumInterfaces(h, null(), &mut list) != 0 {
            WlanCloseHandle(h, null()); // stale handle (service restarted): reopen next time
            WLAN.set(0);
        } else {
            let l = &*list;
            for i in 0..l.dwNumberOfItems as usize {
                let ifc = &*l.InterfaceInfo.as_ptr().add(i);
                let (mut size, mut data) = (0u32, null_mut());
                joining |= ifc.isState == wlan_interface_state_associating || ifc.isState == wlan_interface_state_authenticating;
                if ifc.isState == wlan_interface_state_connected && WlanQueryInterface(h, &ifc.InterfaceGuid, wlan_intf_opcode_current_connection, null(), &mut size, &mut data, null_mut()) == 0 {
                    let a = &(*(data as *const WLAN_CONNECTION_ATTRIBUTES)).wlanAssociationAttributes;
                    q = a.wlanSignalQuality as i32;
                    ssid = String::from_utf8_lossy(&a.dot11Ssid.ucSSID[..(a.dot11Ssid.uSSIDLength as usize).min(32)]).into_owned();
                    WlanFreeMemory(data);
                }
            }
            WlanFreeMemory(list as *const _);
        }
    }
    WIFI.set(q);
    JOINING.set(joining);
    SSID.set(ssid);
    // Windows' own verdict (the same one its tray icon shows): a connection without internet is not "online"
    use windows_sys::Win32::{NetworkManagement::IpHelper::GetNetworkConnectivityHint, Networking::WinSock::{NL_NETWORK_CONNECTIVITY_HINT, NetworkConnectivityLevelHintConstrainedInternetAccess, NetworkConnectivityLevelHintInternetAccess}};
    let mut hint: NL_NETWORK_CONNECTIVITY_HINT = zeroed();
    // a failed call (old Windows) counts as online: never show "no internet" on a guess
    INET.set(GetNetworkConnectivityHint(&mut hint) != 0 || {
        let lvl = hint.ConnectivityLevel;
        lvl == NetworkConnectivityLevelHintInternetAccess || lvl == NetworkConnectivityLevelHintConstrainedInternetAccess
    });
}

/// What the network icon shows.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Link {
    Wifi(i32),      // on wifi with internet, signal 0..100
    NoInternet(i32), // on wifi, but no way out
    Cable,          // internet over ethernet
    Joining,        // not connected yet: a wifi network is being joined
    Offline,        // no connection at all
}

pub(crate) fn net() -> Link {
    match (WIFI.get(), INET.get()) {
        (q, true) if q >= 0 => Link::Wifi(q),
        (q, false) if q >= 0 => Link::NoInternet(q),
        (_, true) => Link::Cable,
        _ if JOINING.get() => Link::Joining,
        _ => Link::Offline,
    }
}

/// Wi-Fi arcs lit for a signal of `q` percent: 1..3.
pub(crate) fn arcs_lit(q: i32) -> i32 {
    if q >= 67 { 3 } else if q >= 34 { 2 } else { 1 }
}

/// The Wi-Fi symbol drawn with anti-aliased arcs: `lit` of the 3 arcs in `on`, the rest in `off`, centered on (cx, cy),
/// `s` px tall. `mark`: 0 none, 1 a cross (not connected), 2 an exclamation mark (connected, no internet).
pub(crate) unsafe fn wifi_glyph(dc: HDC, cx: i32, cy: i32, s: i32, lit: i32, on: u32, off: u32, mark: u8) {
    let mut g: *mut GpGraphics = null_mut();
    if GdipCreateFromHDC(dc, &mut g) != 0 || g.is_null() {
        return;
    }
    GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
    let argb = |c: u32| 0xFF00_0000 | (c & 255) << 16 | (c & 0xFF00) | (c >> 16 & 255);
    let pen = |c: u32, w: f32| {
        let mut p: *mut GpPen = null_mut();
        GdipCreatePen1(argb(c), w, UnitPixel, &mut p);
        GdipSetPenStartCap(p, LineCapRound);
        GdipSetPenEndCap(p, LineCapRound);
        p
    };
    let (sf, w) = (s as f32, if line_icons() { (s as f32 * 0.1).max(1.3) } else { (s as f32 * 0.12).max(1.6) });
    let (bx, by) = (cx as f32, cy as f32 + sf * 0.36); // the point the arcs radiate from
    for i in 0..3 {
        let r = sf * (0.30 + 0.25 * i as f32);
        let p = pen(if i < lit { on } else { off }, w);
        GdipDrawArc(g, p, bx - r, by - r, 2.0 * r, 2.0 * r, 225.0, 90.0);
        GdipDeletePen(p);
    }
    let mut br: *mut GpSolidFill = null_mut();
    GdipCreateSolidFill(argb(if lit > 0 { on } else { off }), &mut br);
    GdipFillEllipse(g, br as *mut GpBrush, bx - w * 0.9, by - w * 0.9, w * 1.8, w * 1.8);
    GdipDeleteBrush(br as *mut GpBrush);
    // the mark sits at the lower right, on a cut-out of the background so it stays readable over the arcs
    if mark > 0 && line_icons() {
        // thin strokes like the other icons: a slash across, or an exclamation mark in the free corner under the arcs
        let p = pen(if mark == 1 { BAD } else { WARN }, w * 1.1);
        if mark == 1 {
            GdipDrawLine(g, p, bx - sf * 0.52, by - sf * 0.86, bx + sf * 0.52, by + sf * 0.1);
        } else {
            GdipDrawLine(g, p, bx + sf * 0.62, by - sf * 0.46, bx + sf * 0.62, by - sf * 0.18);
            GdipDrawLine(g, p, bx + sf * 0.62, by + sf * 0.02, bx + sf * 0.62, by + sf * 0.04);
        }
        GdipDeletePen(p);
        GdipDeleteGraphics(g);
        return;
    }
    let (mx, my, mr) = (bx + sf * 0.42, by - sf * 0.08, sf * 0.26);
    if mark > 0 {
        let mut bg: *mut GpSolidFill = null_mut();
        GdipCreateSolidFill(argb(pal().bg), &mut bg);
        GdipFillEllipse(g, bg as *mut GpBrush, mx - mr, my - mr, 2.0 * mr, 2.0 * mr);
        GdipDeleteBrush(bg as *mut GpBrush);
        let c = if mark == 1 { on } else { WARN };
        let p = pen(c, (w * 0.8).max(1.4));
        let d = mr * 0.5;
        if mark == 1 {
            GdipDrawLine(g, p, mx - d, my - d, mx + d, my + d);
            GdipDrawLine(g, p, mx + d, my - d, mx - d, my + d);
        } else {
            GdipDrawLine(g, p, mx, my - d * 1.2, mx, my + d * 0.2);
            GdipDrawLine(g, p, mx, my + d * 0.95, mx, my + d * 1.0);
        }
        GdipDeletePen(p);
    }
    GdipDeleteGraphics(g);
}

/// Hover key of the tray chevron's turn.
pub(crate) const TF_KEY: i32 = -13;

/// A chevron of half-width `s` around (cx, cy), drawn with round line ends; `deg` 0 points up, 180 down.
pub(crate) unsafe fn chevron(dc: HDC, cx: i32, cy: i32, s: i32, deg: f32, c: u32) {
    let mut g: *mut GpGraphics = null_mut();
    if GdipCreateFromHDC(dc, &mut g) != 0 || g.is_null() {
        return;
    }
    GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
    let mut p: *mut GpPen = null_mut();
    GdipCreatePen1(0xFF00_0000 | (c & 255) << 16 | (c & 0xFF00) | (c >> 16 & 255), if line_icons() { (s as f32 * 0.22).max(1.3) } else { (s as f32 * 0.32).max(1.4) }, UnitPixel, &mut p);
    GdipSetPenStartCap(p, LineCapRound);
    GdipSetPenEndCap(p, LineCapRound);
    let (sn, cs) = deg.to_radians().sin_cos();
    let pt = |x: f32, y: f32| (cx as f32 + x * cs - y * sn, cy as f32 + x * sn + y * cs);
    let (sf, h) = (s as f32, s as f32 * 0.5);
    let (a, b, d) = (pt(-sf, h), pt(0.0, -h), pt(sf, h));
    GdipDrawLine(g, p, a.0, a.1, b.0, b.1);
    GdipDrawLine(g, p, b.0, b.1, d.0, d.1);
    GdipDeletePen(p);
    GdipDeleteGraphics(g);
}

/// The network icon of the bar and the panels, for the current state.
pub(crate) unsafe fn net_icon(dc: HDC, cx: i32, cy: i32, s: i32, f_glyph: HFONT) {
    let s = if line_icons() { s * 13 / 17 } else { s };
    let (on, off) = (icon_ink(), mix(pal().dim, pal().bg, 0.45));
    match net() {
        Link::Wifi(q) => wifi_glyph(dc, cx, cy, s, arcs_lit(q), on, off, 0),
        Link::NoInternet(q) => wifi_glyph(dc, cx, cy, s, arcs_lit(q), mix(on, pal().bg, 0.3), off, 2),
        Link::Joining => {
            // the arcs fill up until it is through; the bar's timer 9 turns the frames
            wifi_glyph(dc, cx, cy, s, (now_ms() / 230 % 4) as i32, on, off, 0)
        }
        Link::Offline => wifi_glyph(dc, cx, cy, s, 0, on, mix(pal().dim, pal().bg, 0.2), 1),
        Link::Cable => {
            SelectObject(dc, f_glyph);
            sym(dc, "\u{E839}", RECT { left: cx - s, top: cy - s, right: cx + s, bottom: cy + s }, on);
        }
    }
}
