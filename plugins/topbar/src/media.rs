//! topbar: what plays now (the media sessions apps publish to Windows), for the buttons in an app's preview.
//! The calls wait on a Windows service, so a thread of its own makes them; the preview reads the last answer.
use std::sync::{
    Mutex, OnceLock,
    mpsc::{Sender, channel},
};
use windows::Media::Control::{GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager, GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status};

/// One app's session. `id` is its app id, or its exe's name (lower-case).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Media {
    pub id: String,
    pub playing: bool,
    pub can: [bool; 3], // previous, play / pause, next
}

static NOW: Mutex<Vec<Media>> = Mutex::new(Vec::new());
static TX: OnceLock<Mutex<Sender<Option<(String, usize)>>>> = OnceLock::new();

fn read(s: &Session) -> windows::core::Result<Media> {
    let info = s.GetPlaybackInfo()?;
    let c = info.Controls()?;
    let on = |r: windows::core::Result<bool>| r.unwrap_or(false);
    Ok(Media {
        id: s.SourceAppUserModelId()?.to_string().to_lowercase(),
        playing: info.PlaybackStatus()? == Status::Playing,
        can: [on(c.IsPreviousEnabled()), on(c.IsPlayPauseToggleEnabled()) || on(c.IsPlayEnabled()) || on(c.IsPauseEnabled()), on(c.IsNextEnabled())],
    })
}

/// The app gets the command and answers when it likes: nothing here waits for it.
fn run(s: &Session, button: usize) -> windows::core::Result<()> {
    match button {
        0 => drop(s.TrySkipPreviousAsync()?),
        1 if s.GetPlaybackInfo()?.Controls()?.IsPlayPauseToggleEnabled()? => drop(s.TryTogglePlayPauseAsync()?),
        1 if s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing => drop(s.TryPauseAsync()?),
        1 => drop(s.TryPlayAsync()?),
        _ => drop(s.TrySkipNextAsync()?),
    }
    Ok(())
}

fn send(m: Option<(String, usize)>) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<Option<(String, usize)>>();
        std::thread::spawn(move || {
            unsafe { let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED); }
            let mut man: Option<Manager> = None;
            while let Ok(first) = rx.recv() {
                if man.is_none() {
                    man = Manager::RequestAsync().and_then(|a| a.join()).ok();
                }
                let list = || -> windows::core::Result<Vec<Session>> { Ok(man.as_ref().ok_or_else(windows::core::Error::empty)?.GetSessions()?.into_iter().collect()) };
                let all = list();
                // what piled up meanwhile: one look answers every "refresh"
                for (id, button) in std::iter::once(first).chain(rx.try_iter()).flatten() {
                    for s in all.iter().flatten() {
                        if s.SourceAppUserModelId().is_ok_and(|x| x.to_string().to_lowercase() == id) {
                            let _ = run(s, button);
                            std::thread::sleep(std::time::Duration::from_millis(120)); // the state follows a moment later
                            break;
                        }
                    }
                }
                *NOW.lock().unwrap() = all.iter().flatten().filter_map(|s| read(s).ok()).collect();
                if all.is_err() {
                    man = None; // the service restarted: asked for again next time
                }
            }
        });
        Mutex::new(tx)
    });
    let _ = tx.lock().unwrap().send(m);
}

/// Looks again what plays; the answer is there a few milliseconds later.
pub fn refresh() {
    send(None);
}

/// Presses button 0 (previous), 1 (play / pause) or 2 (next) of session `id`.
pub fn press(id: &str, button: usize) {
    send(Some((id.to_string(), button)));
}

/// The session of an app known by its exe and app ids. Browsers have none, like on the Windows taskbar: their session
/// belongs to one tab, not to the window.
pub fn of(exe: &str, ids: &[&str]) -> Option<Media> {
    let name = exe.rsplit('\\').next().unwrap_or("");
    if ["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe"].contains(&name) {
        return None;
    }
    NOW.lock().unwrap().iter().find(|m| !m.id.is_empty() && (m.id == name || ids.contains(&m.id.as_str()))).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_belongs_to_its_app() {
        let m = |id: &str| Media { id: id.into(), ..Default::default() };
        *NOW.lock().unwrap() = vec![m("spotifyab.spotifymusic_zpdnekdrzrea0!spotify"), m("vlc.exe"), m("chrome")];
        assert!(of(r"c:\x\spotify.exe", &["", "spotifyab.spotifymusic_zpdnekdrzrea0!spotify"]).is_some());
        assert!(of(r"c:\vlc\vlc.exe", &[""]).is_some());
        assert!(of(r"c:\x\chrome.exe", &["chrome"]).is_none());
        assert!(of(r"c:\x\notepad.exe", &[""]).is_none());
    }
}
