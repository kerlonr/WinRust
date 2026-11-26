//! topbar: the sound page of the quick panel, as in Windows 11: the outputs (pick the default one) and the volume of each
//! app playing (shares main.rs through `super::*`).
use super::*;
use windows::{
    core::{GUID, HRESULT, Interface, IUnknown},
    Win32::{
        Foundation::{PROPERTYKEY, S_OK},
        Media::Audio::{AudioSessionStateExpired, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, ISimpleAudioVolume},
        System::Com::{CoTaskMemFree, STGM_READ, StructuredStorage::PropVariantToStringAlloc},
    },
};

pub(crate) struct Dev {
    pub id: String,
    pub name: String,
    pub on: bool, // the default output
}

pub(crate) struct App {
    key: String, // what groups its sessions: the program file ("" for system sounds)
    pub name: String,
    pub icon: HICON, // null: system sounds (drawn as a glyph)
    pub vol: f32,
    pub mute: bool,
    ctl: Vec<ISimpleAudioVolume>,
}

thread_local! {
    pub(crate) static MIX: RefCell<(Vec<Dev>, Vec<App>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

/// Apps shown at most: the panel stays on the screen.
pub(crate) const MAX_APPS: usize = 8;

pub(crate) unsafe fn device_id(d: &IMMDevice) -> Option<String> {
    let p = d.GetId().ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s
}

unsafe fn device_name(d: &IMMDevice) -> Option<String> {
    let ps = d.OpenPropertyStore(STGM_READ).ok()?;
    let key = PROPERTYKEY { fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0), pid: 14 }; // PKEY_Device_FriendlyName
    let p = PropVariantToStringAlloc(&ps.GetValue(&key).ok()?).ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as *const _));
    s.filter(|s| !s.is_empty())
}

/// Full path of a process's program, lower case.
unsafe fn exe_of(pid: u32) -> Option<String> {
    let p = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
    if p.is_null() {
        return None;
    }
    let (mut b, mut n) = ([0u16; 520], 520u32);
    let ok = QueryFullProcessImageNameW(p, 0, b.as_mut_ptr(), &mut n);
    CloseHandle(p);
    (ok != 0).then(|| String::from_utf16_lossy(&b[..n as usize]).to_lowercase())
}

/// Name and icon of the app behind a session; a packaged app (WhatsApp, Spotify from the Store) by its Start menu entry.
unsafe fn who(pid: u32, exe: &str) -> (String, HICON) {
    let px = crate::icons::load_px();
    if exe.contains(r"\windowsapps\")
        && let Some(id) = aumid_of(pid)
    {
        let item = format!(r"shell:AppsFolder\{id}");
        if let Some(n) = crate::tray::shell_name(&item) {
            return (n, cached_icon(&item, px));
        }
    }
    (crate::tray::app_name(exe, null_mut()), cached_icon(exe, px))
}

/// Reads the outputs and the apps that have sound on the default one.
pub(crate) unsafe fn refresh() {
    let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return };
    let def = en.GetDefaultAudioEndpoint(eRender, eConsole).ok();
    let def_id = def.as_ref().and_then(|d| device_id(d));
    let mut devs = vec![];
    if let Ok(c) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) {
        for i in 0..c.GetCount().unwrap_or(0) {
            let Ok(d) = c.Item(i) else { continue };
            let Some(id) = device_id(&d) else { continue };
            devs.push(Dev { on: def_id.as_ref() == Some(&id), name: device_name(&d).unwrap_or_else(|| id.clone()), id });
        }
    }
    let mut apps: Vec<App> = vec![];
    let sessions = def.and_then(|d| d.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None).ok()).and_then(|m| m.GetSessionEnumerator().ok());
    if let Some(e) = sessions {
        for i in 0..e.GetCount().unwrap_or(0) {
            let Ok(s) = e.GetSession(i) else { continue };
            if s.GetState().is_ok_and(|st| st == AudioSessionStateExpired) {
                continue;
            }
            let (Ok(s2), Ok(v)) = (s.cast::<IAudioSessionControl2>(), s.cast::<ISimpleAudioVolume>()) else { continue };
            let system = s2.IsSystemSoundsSession() == S_OK;
            let pid = s2.GetProcessId().unwrap_or(0);
            let exe = if system { Some(String::new()) } else { exe_of(pid) };
            let Some(key) = exe else { continue }; // its process is gone
            if let Some(a) = apps.iter_mut().find(|a| a.key == key) {
                a.ctl.push(v);
                continue;
            }
            let (name, icon) = if system { (t!("Системные звуки", "System sounds", ja: "システム音", pt: "Sons do sistema", zh: "系统声音").to_string(), null_mut()) } else { who(pid, &key) };
            let (vol, mute) = (v.GetMasterVolume().unwrap_or(1.0), v.GetMute().is_ok_and(|m| m.as_bool()));
            apps.push(App { key, name, icon, vol, mute, ctl: vec![v] });
        }
    }
    // Windows 11's order: system sounds last, the rest by name
    apps.sort_by_key(|a| (a.key.is_empty(), a.name.to_lowercase()));
    apps.truncate(MAX_APPS);
    MIX.set((devs, apps));
}

/// Volume (0..1) and/or mute of app `i`; moving its volume turns its sound back on.
pub(crate) unsafe fn set_app(i: usize, vol: Option<f32>, mute: Option<bool>) {
    MIX.with_borrow_mut(|m| {
        let Some(a) = m.1.get_mut(i) else { return };
        let mute = mute.or(vol.map(|_| false));
        for c in &a.ctl {
            if let Some(v) = vol {
                let _ = c.SetMasterVolume(v.clamp(0.0, 1.0), null());
            }
            if let Some(x) = mute {
                let _ = c.SetMute(x, null());
            }
        }
        a.vol = vol.map_or(a.vol, |v| v.clamp(0.0, 1.0));
        a.mute = mute.unwrap_or(a.mute);
    });
}

/// Makes `id` the default output for every role, as the sound settings do.
// shortcut: IPolicyConfig is undocumented (the same one EarTrumpet and SoundSwitch use); if a Windows update drops it, open ms-settings:sound instead.
pub(crate) unsafe fn set_default(id: &str) {
    const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
    const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);
    let Ok(u) = CoCreateInstance::<_, IUnknown>(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL) else { return };
    let mut p: *mut core::ffi::c_void = null_mut();
    if u.query(&IID_POLICY_CONFIG, &mut p).is_err() || p.is_null() {
        return;
    }
    let vt = *(p as *const *const usize);
    // IPolicyConfig: 3 IUnknown methods, 10 more, then SetDefaultEndpoint(id, role)
    let set: unsafe extern "system" fn(*mut core::ffi::c_void, PCWSTR, i32) -> HRESULT = std::mem::transmute(*vt.add(13));
    let release: unsafe extern "system" fn(*mut core::ffi::c_void) -> u32 = std::mem::transmute(*vt.add(2));
    let w = wide(id);
    for role in 0..3 {
        let _ = set(p, PCWSTR(w.as_ptr()), role); // console, multimedia, communications
    }
    release(p);
    read_volume();
    refresh();
}
