//! The Start panel of the Delphos style: a grid of apps, and one search over apps, settings, files (the rust-search index),
//! sums and the web. (A child of fly.rs: it shares its items through `super::*`.)
use super::*;
use std::sync::atomic::AtomicU32;

pub(super) const W: i32 = 600;
pub(super) const H: i32 = 572;
const COLS: usize = 6;
const GRID_ROWS: usize = 4;
const CELL: (i32, i32) = (92, 96);
const TOP: i32 = 112; // where the grid starts
const LIST_TOP: i32 = 76;
const ROW: i32 = 46;
const LIST_ROWS: usize = 9;
const PW: i32 = 208; // the power menu: width and row height
const PROW: i32 = 40;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum What {
    App,
    Setting,
    File,
    Folder,
    Sum,
    Web,
}

#[derive(Clone, Debug)]
pub(super) struct Item {
    pub what: What,
    pub name: String,
    pub sub: String,
    pub path: String, // what opens it (the answer itself for a sum)
}

static FILES: Mutex<(String, Vec<(String, bool)>)> = Mutex::new((String::new(), Vec::new())); // query, its files (path, is a folder)
static GEN: AtomicU32 = AtomicU32::new(0);

thread_local! {
    pub(super) static ALL: Cell<bool> = const { Cell::new(false) }; // the grid lists every app, not just the pinned / most used ones
    static WAITING: RefCell<std::collections::HashSet<Vec<u16>>> = RefCell::new(Default::default()); // icons asked for
    pub(super) static PMENU: Cell<u64> = const { Cell::new(0) }; // when the power menu opened, 0 = closed
}

// ---------- matching ----------

/// Lower case, without accents: "Configurações" is found by "configuracoes".
pub(super) fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// Windows settings: (Portuguese, English, more words to find it by, address).
const SETTINGS: [(&str, &str, &str, &str); 36] = [
    ("Tela", "Display", "monitor resolucao resolution brilho brightness escala scale", "ms-settings:display"),
    ("Som", "Sound", "audio volume microfone microphone alto falante speaker", "ms-settings:sound"),
    ("Notificações", "Notifications", "avisos focus nao perturbe", "ms-settings:notifications"),
    ("Energia e bateria", "Power & battery", "suspender sleep economia", "ms-settings:powersleep"),
    ("Armazenamento", "Storage", "disco espaco disk space limpeza", "ms-settings:storagesense"),
    ("Multitarefa", "Multitasking", "janelas snap", "ms-settings:multitasking"),
    ("Área de transferência", "Clipboard", "copiar colar historico", "ms-settings:clipboard"),
    ("Área de trabalho remota", "Remote Desktop", "rdp acesso remoto", "ms-settings:remotedesktop"),
    ("Sobre o computador", "About", "sistema system versao especificacoes specs nome", "ms-settings:about"),
    ("Bluetooth e dispositivos", "Bluetooth & devices", "fone headset parear pair", "ms-settings:bluetooth"),
    ("Impressoras e scanners", "Printers & scanners", "imprimir print", "ms-settings:printers"),
    ("Mouse", "Mouse", "ponteiro pointer cursor", "ms-settings:mousetouchpad"),
    ("Touchpad", "Touchpad", "gestos gestures", "ms-settings:devices-touchpad"),
    ("Digitação", "Typing", "teclado keyboard autocorrecao", "ms-settings:typing"),
    ("USB", "USB", "", "ms-settings:usb"),
    ("Wi‑Fi", "Wi‑Fi", "wifi rede sem fio network internet", "ms-settings:network-wifi"),
    ("Rede e internet", "Network & internet", "ethernet ip status", "ms-settings:network-status"),
    ("VPN", "VPN", "rede privada", "ms-settings:network-vpn"),
    ("Proxy", "Proxy", "", "ms-settings:network-proxy"),
    ("Hotspot móvel", "Mobile hotspot", "compartilhar internet", "ms-settings:network-mobilehotspot"),
    ("Plano de fundo", "Background", "papel de parede wallpaper", "ms-settings:personalization-background"),
    ("Cores", "Colors", "tema escuro dark mode claro", "ms-settings:colors"),
    ("Temas", "Themes", "personalizar", "ms-settings:themes"),
    ("Tela de bloqueio", "Lock screen", "bloquear", "ms-settings:lockscreen"),
    ("Fontes", "Fonts", "letras", "ms-settings:fonts"),
    ("Aplicativos instalados", "Installed apps", "desinstalar uninstall programas remover", "ms-settings:appsfeatures"),
    ("Aplicativos padrão", "Default apps", "navegador padrao browser", "ms-settings:defaultapps"),
    ("Inicialização", "Startup", "iniciar com o windows", "ms-settings:startupapps"),
    ("Contas", "Your info", "usuario user conta senha", "ms-settings:yourinfo"),
    ("Opções de entrada", "Sign-in options", "senha pin digital password", "ms-settings:signinoptions"),
    ("Data e hora", "Date & time", "relogio clock fuso timezone", "ms-settings:dateandtime"),
    ("Idioma e região", "Language & region", "teclado formato", "ms-settings:regionlanguage"),
    ("Acessibilidade", "Accessibility", "lupa narrador contraste", "ms-settings:easeofaccess"),
    ("Privacidade e segurança", "Privacy & security", "permissoes camera localizacao", "ms-settings:privacy"),
    ("Segurança do Windows", "Windows Security", "antivirus defender firewall", "ms-settings:windowsdefender"),
    ("Windows Update", "Windows Update", "atualizar atualizacao update", "ms-settings:windowsupdate"),
];

fn setting_name(s: &(&'static str, &'static str, &str, &str)) -> &'static str {
    if matches!(winrust_common::lang!(), winrust_common::Lang::Pt) { s.0 } else { s.1 }
}

/// How well a settings page answers the (folded) query; smaller is better.
fn setting_score(s: &(&'static str, &'static str, &str, &str), q: &str) -> Option<u8> {
    let names = [fold(s.0), fold(s.1)];
    names.iter().filter_map(|n| score(n, q)).min().or_else(|| s.2.split(' ').any(|w| q.len() >= 3 && w.starts_with(q)).then_some(4))
}

/// How well a file answers the (folded) query: bigger is better, None = leave it out. The user's own folders come first, what
/// programs keep for themselves last.
pub(super) fn file_score(path: &str, q: &str, home: &str) -> Option<i32> {
    let p = fold(path);
    let name = p.rsplit('\\').next().unwrap_or("");
    let stem = name.rsplit_once('.').map_or(name, |x| x.0);
    if p.contains("\\$recycle.bin\\") || p.contains("\\winsxs\\") || (name.ends_with(".lnk") && (p.contains("\\start menu\\") || p.contains("\\quick launch\\"))) {
        return None;
    }
    let mut s = if stem == q || name == q { 100 } else if name.starts_with(q) { 70 } else if name.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(q)) { 50 } else { 25 };
    let home = fold(home);
    if !home.is_empty() && p.starts_with(&home) {
        s += 30;
        if ["\\desktop\\", "\\documents\\", "\\downloads\\", "\\pictures\\", "\\videos\\", "\\music\\", "\\onedrive"].iter().any(|d| p.contains(d)) {
            s += 25;
        }
    } else if p.contains("\\users\\") {
        s -= 25; // somebody else's
    }
    for junk in ["\\windows\\", "\\programdata\\", "\\appdata\\", "\\program files", "\\node_modules\\", "\\.git\\", "\\.cargo\\", "\\target\\", "\\cache", "\\temp\\", "\\windowsapps\\", "\\__pycache__\\", "\\site-packages\\"] {
        if p.contains(junk) {
            s -= 60;
        }
    }
    if p.contains("\\.git\\") || name.starts_with(".~") || name.starts_with("~$") {
        return None; // a program's own bookkeeping
    }
    Some(s - 2 * p.matches('\\').count() as i32)
}

/// A sum ("12,5*4", "(2+3)^2"): None unless it is one.
pub(super) fn sum(q: &str) -> Option<f64> {
    if !q.chars().any(|c| c.is_ascii_digit()) || !q.chars().skip(1).any(|c| "+-*/^%x×÷".contains(c)) {
        return None;
    }
    let t: Vec<char> = q.chars().filter(|c| !c.is_whitespace()).map(|c| match c { ',' => '.', 'x' | '×' => '*', '÷' => '/', c => c }).collect();
    struct P<'a>(&'a [char], usize, u8); // text, position, nesting
    impl P<'_> {
        fn at(&self) -> char {
            self.0.get(self.1).copied().unwrap_or('\0')
        }
        fn add(&mut self) -> Option<f64> {
            let mut v = self.mul()?;
            while matches!(self.at(), '+' | '-') {
                let op = self.at();
                self.1 += 1;
                let r = self.mul()?;
                v = if op == '+' { v + r } else { v - r };
            }
            Some(v)
        }
        fn mul(&mut self) -> Option<f64> {
            let mut v = self.pow()?;
            while matches!(self.at(), '*' | '/' | '%') {
                let op = self.at();
                self.1 += 1;
                let r = self.pow()?;
                v = match op { '*' => v * r, '/' => v / r, _ => v % r };
            }
            Some(v)
        }
        fn pow(&mut self) -> Option<f64> {
            let v = self.one()?;
            if self.at() != '^' {
                return Some(v);
            }
            self.1 += 1;
            self.2 += 1;
            if self.2 > 40 { return None }
            Some(v.powf(self.pow()?))
        }
        fn one(&mut self) -> Option<f64> {
            self.2 += 1;
            if self.2 > 40 {
                return None; // "((((((...": no stack to spare
            }
            let v = match self.at() {
                '-' => { self.1 += 1; -self.one()? }
                '(' => {
                    self.1 += 1;
                    let v = self.add()?;
                    if self.at() != ')' { return None }
                    self.1 += 1;
                    v
                }
                _ => {
                    let a = self.1;
                    while self.at().is_ascii_digit() || self.at() == '.' { self.1 += 1 }
                    self.0[a..self.1].iter().collect::<String>().parse().ok()?
                }
            };
            self.2 -= 1;
            Some(v)
        }
    }
    let mut p = P(&t, 0, 0);
    let v = p.add()?;
    (p.1 == t.len() && v.is_finite()).then_some(v)
}

fn num(v: f64) -> String {
    let s = format!("{v:.8}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if matches!(winrust_common::lang!(), winrust_common::Lang::Pt | winrust_common::Lang::Ru) { s.replace('.', ",") } else { s }
}

fn percent(q: &str) -> String {
    q.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn typed() -> String {
    QUERY.with_borrow(|q| q.trim().to_string())
}

pub(super) fn searching() -> bool {
    !typed().is_empty()
}

fn app_item(a: &(String, Vec<u16>)) -> Item {
    Item { what: What::App, name: a.0.clone(), sub: String::new(), path: wstr(&a.1) }
}

/// What the panel shows now, in order.
pub(super) fn items() -> Vec<Item> {
    let raw = typed();
    if raw.is_empty() {
        return grid_items();
    }
    let q = fold(&raw);
    let fq = fold(&flip_layout(&raw.to_lowercase()));
    let mut out = vec![];
    if let Some(v) = sum(&raw) {
        out.push(Item { what: What::Sum, name: num(v), sub: t!("Enter — скопировать", "Enter copies it", ja: "Enter でコピー", pt: "Enter copia o resultado", zh: "Enter 复制").into(), path: num(v) });
    }
    {
        let apps = APPS.lock().unwrap();
        let mut v: Vec<(u8, u32, usize)> = apps
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let n = fold(&a.0);
                Some((score(&n, &q).or_else(|| score(&n, &fq).map(|s| s + 4))?, u32::MAX - uses(&n), i))
            })
            .collect();
        v.sort_unstable();
        out.extend(v.into_iter().take(5).map(|x| app_item(&apps[x.2])));
    }
    let mut v: Vec<(u8, usize)> = SETTINGS.iter().enumerate().filter_map(|(i, s)| Some((setting_score(s, &q)?, i))).collect();
    v.sort_unstable();
    let settings = t!("Параметры", "Settings", ja: "設定", pt: "Configurações", zh: "设置");
    out.extend(v.into_iter().take(3).map(|(_, i)| Item { what: What::Setting, name: setting_name(&SETTINGS[i]).into(), sub: settings.into(), path: SETTINGS[i].3.into() }));
    {
        let f = FILES.lock().unwrap();
        // while a newer search runs, the last answer stays as far as it still fits
        out.extend(f.1.iter().filter(|(p, _)| f.0 == raw || fold(p.rsplit('\\').next().unwrap_or("")).contains(&q)).map(|(p, dir)| {
            let (folder, name) = p.rsplit_once('\\').unwrap_or(("", p));
            Item { what: if *dir { What::Folder } else { What::File }, name: name.into(), sub: folder.into(), path: p.clone() }
        }));
    }
    out.push(Item { what: What::Web, name: raw.clone(), sub: t!("Искать в интернете", "Search the web", ja: "ウェブを検索", pt: "Pesquisar na web", zh: "在网上搜索").into(), path: format!("https://www.google.com/search?q={}", percent(&raw)) });
    out
}

fn grid_items() -> Vec<Item> {
    let pinned = PINNED.lock().unwrap().clone();
    let apps = APPS.lock().unwrap();
    if ALL.get() {
        return apps.iter().map(app_item).collect();
    }
    if !pinned.is_empty() {
        return pinned
            .iter()
            .map(|p| apps.iter().find(|a| wstr(&a.1).eq_ignore_ascii_case(p)).map(app_item).unwrap_or_else(|| Item { what: What::App, name: std::path::Path::new(p).file_stem().unwrap_or_default().to_string_lossy().into(), sub: String::new(), path: p.clone() }))
            .collect();
    }
    let mut v: Vec<(u32, usize)> = apps.iter().enumerate().map(|(i, a)| (u32::MAX - uses(&a.0.to_lowercase()), i)).collect();
    v.sort_unstable();
    v.into_iter().take(COLS * GRID_ROWS).map(|x| app_item(&apps[x.1])).collect()
}

/// Asks the file index for the query, off the UI thread.
pub(super) fn search(hw: HWND) {
    let q = typed();
    let me = GEN.fetch_add(1, Relaxed) + 1;
    if q.chars().count() < 2 {
        FILES.lock().unwrap().1.clear();
        return;
    }
    let hw = hw as isize;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(35));
        if GEN.load(Relaxed) != me {
            return; // typed on
        }
        let found = winrust_common::search::ask_all(&q);
        if GEN.load(Relaxed) != me {
            return;
        }
        let (f, home) = (fold(&q), std::env::var("USERPROFILE").unwrap_or_default());
        let mut v: Vec<(i32, String)> = found.into_iter().filter_map(|p| Some((-file_score(&p, f.split(' ').next().unwrap_or(&f), &home)?, p))).collect();
        v.sort();
        let list = v.into_iter().take(12).map(|(_, p)| { let d = std::path::Path::new(&p).is_dir(); (p, d) }).collect();
        *FILES.lock().unwrap() = (q, list);
        unsafe { InvalidateRect(hw as HWND, null(), 0) };
    });
}

// ---------- geometry ----------

/// Lines that scroll: rows of the grid, or of the list.
fn lines(n: usize) -> (usize, usize) {
    if searching() { (n, LIST_ROWS) } else { (n.div_ceil(COLS), GRID_ROWS) }
}

pub(super) fn scroll(up: bool) {
    let (n, shown) = lines(items().len());
    let step = if searching() { 2 } else { 1 };
    SCROLL.set(if up { SCROLL.get().saturating_sub(step) } else { (SCROLL.get() + step).min(n.saturating_sub(shown)) });
}

fn button(i: i32) -> RECT {
    // the footer's round buttons, from the right: power, settings, Explorer
    r(W - 12 - 38 - 42 * i, H - 50, W - 12 - 42 * i, H - 12)
}

fn power_rect() -> RECT {
    r(W - 12 - PW, H - 58 - 12 - 4 * PROW, W - 12, H - 58)
}

pub(super) fn rects() -> Vec<(RECT, Act)> {
    let (n, s) = (items().len(), SCROLL.get());
    let mut v = vec![];
    if searching() {
        v.extend((0..LIST_ROWS).filter(|i| s + i < n).map(|i| (r(12, LIST_TOP + ROW * i as i32, W - 12, LIST_TOP + ROW * (i as i32 + 1) - 2), Act::Hit(s + i))));
    } else {
        for i in s * COLS..((s + GRID_ROWS) * COLS).min(n) {
            let (c, l) = ((i % COLS) as i32, (i / COLS - s) as i32);
            let x = (W - COLS as i32 * CELL.0) / 2 + c * CELL.0;
            v.push((r(x + 2, TOP + l * CELL.1 + 2, x + CELL.0 - 2, TOP + (l + 1) * CELL.1 - 2), Act::Hit(i)));
        }
        v.push((r(W - 24 - 130, 72, W - 24, 102), Act::AllApps));
    }
    v.push((r(W - 54, 22, W - 22, 52), Act::Keyboard));
    v.push((button(2), Act::Explorer));
    v.push((button(1), Act::Settings("ms-settings:")));
    v.push((button(0), Act::PowerMenu));
    if PMENU.get() != 0 {
        let p = power_rect();
        v.extend((0..4).map(|i| (RECT { left: p.left + sc(6), right: p.right - sc(6), top: p.top + sc(6 + PROW * i), bottom: p.top + sc(6 + PROW * (i + 1)) }, Act::Power(i as u8))));
    }
    v
}

// ---------- actions ----------

/// Opens item `i`. false = there is none.
pub(super) unsafe fn run(i: usize) -> bool {
    let Some(it) = items().into_iter().nth(i) else { return false };
    if PREVIEW.load(Relaxed) {
        return true;
    }
    match it.what {
        What::Sum => copy(&it.path),
        _ => {
            if it.path.to_lowercase().starts_with("shell:") {
                bump_name(&it.name); // open() counts only .lnk files
            }
            // off the UI thread: the shell takes its time
            thread::spawn(move || {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                open(&wide(&it.path));
            });
        }
    }
    true
}

unsafe fn copy(s: &str) {
    use windows_sys::Win32::System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
    };
    let w = wide(s);
    if OpenClipboard(null_mut()) == 0 {
        return;
    }
    EmptyClipboard();
    let m = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2);
    if !m.is_null() {
        let p = GlobalLock(m) as *mut u16;
        std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
        GlobalUnlock(m);
        SetClipboardData(13, m as _); // CF_UNICODETEXT: the clipboard owns it now
    }
    CloseClipboard();
}

/// This computer, in whatever handles folders here.
pub(super) unsafe fn explorer() {
    if PREVIEW.load(Relaxed) {
        return;
    }
    thread::spawn(|| {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let rx = PathBuf::from(std::env::var_os("ProgramFiles").unwrap_or_default()).join(r"rust-explorer\rust-explorer.exe");
        if rx.is_file() {
            open(&wide(&rx.to_string_lossy()));
        } else {
            open(&wide("shell:MyComputerFolder"));
        }
    });
}

/// Arrow keys.
pub(super) unsafe fn arrow(vk: u16) {
    let n = items().len();
    if n == 0 {
        return;
    }
    let cols = if searching() { 1 } else { COLS };
    let s = SEL.get().min(n - 1);
    let s = match vk {
        VK_DOWN => (s + cols).min(n - 1),
        VK_UP => s.saturating_sub(cols),
        VK_RIGHT if cols > 1 => (s + 1).min(n - 1),
        VK_LEFT if cols > 1 => s.saturating_sub(1),
        _ => s,
    };
    SEL.set(s);
    let shown = lines(n).1;
    let line = s / cols;
    SCROLL.set(SCROLL.get().min(line).max((line + 1).saturating_sub(shown)));
}

/// Right click on item `i`.
pub(super) unsafe fn menu(hw: HWND, i: usize, x: i32, y: i32) {
    let Some(it) = items().into_iter().nth(i) else { return };
    let file = it.path.contains(":\\");
    let l = it.path.to_lowercase();
    let runs = l.ends_with(".lnk") || l.ends_with(".exe");
    let m = CreatePopupMenu();
    let add = |id: usize, s: &str| AppendMenuW(m, MF_STRING, id, wide(s).as_ptr());
    add(1, t!("Открыть", "Open", ja: "開く", pt: "Abrir", zh: "打开"));
    if runs {
        add(2, t!("Запуск от имени администратора", "Run as administrator", ja: "管理者として実行", pt: "Executar como administrador", zh: "以管理员身份运行"));
    }
    if file {
        add(3, t!("Расположение файла", "Open file location", ja: "ファイルの場所を開く", pt: "Abrir local do arquivo", zh: "打开文件所在位置"));
    }
    if it.what == What::App || file {
        add(4, if start_pinned(&it.path) { t!("Убрать из Пуска", "Remove from Start", ja: "スタートから削除", pt: "Remover do Iniciar", zh: "从开始菜单移除") } else { t!("Добавить в Пуск", "Add to Start", ja: "スタートに追加", pt: "Adicionar ao Iniciar", zh: "添加到开始菜单") });
    }
    let mut pt = POINT { x, y };
    ClientToScreen(hw, &mut pt);
    let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, hw, null());
    DestroyMenu(m);
    match cmd {
        1 => _ = run(i),
        2 => _ = ShellExecuteW(null_mut(), wide("runas").as_ptr(), wide(&it.path).as_ptr(), null(), null(), SW_SHOWNORMAL),
        3 => winrust_common::reveal(&it.path),
        4 => return start_toggle(&it.path), // stays open: the list just changed
        _ => return,
    }
    close();
}

// ---------- painting ----------

pub(super) struct Fonts {
    pub txt: HFONT,
    pub small: HFONT,
    pub glyph: HFONT,
    pub semi: HFONT,
}

type Ask = (Vec<u16>, What, String, isize); // key, what it is, path, the panel to repaint
static ASK: Mutex<Option<std::sync::mpsc::Sender<Ask>>> = Mutex::new(None);
static DONE: Mutex<Vec<(Vec<u16>, isize)>> = Mutex::new(Vec::new()); // loaded icons on their way back

/// Loads icons one after another on a thread of its own: some take the shell a fifth of a second, and typing must not wait.
unsafe fn load_icons(rx: std::sync::mpsc::Receiver<Ask>) {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    for (key, what, path, hw) in rx {
        let w = wide(&path);
        let mut ic = null_mut();
        for n in 0..3 {
            // the shell sometimes has no answer the first time
            if n > 0 {
                thread::sleep(std::time::Duration::from_millis(60));
            }
            ic = if what == What::App { lnk_icon(&w, &lnk_target(&w)) } else { item_icon(&path, load_px()) };
            if !ic.is_null() || what != What::App {
                break;
            }
        }
        if ic.is_null() && what == What::File {
            // not ours to read (another user's): what its kind of file looks like
            let mut sfi: SHFILEINFOW = zeroed();
            if SHGetFileInfoW(w.as_ptr(), 0x80, &mut sfi, size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES) != 0 {
                ic = sfi.hIcon;
            }
        }
        forget_icon(ic); // the smooth copy belongs to the thread that draws
        DONE.lock().unwrap().push((key, ic as isize));
        InvalidateRect(hw as HWND, null(), 0);
    }
}

/// The icon of an item; null while it loads, or when it has none.
unsafe fn icon(it: &Item) -> HICON {
    let l = it.path.to_lowercase();
    let key = match it.what {
        What::Folder => "kind:folder".to_string(),
        What::File if !(l.ends_with(".exe") || l.ends_with(".lnk") || l.ends_with(".ico") || l.ends_with(".url")) => format!("kind:{}", l.rsplit('\\').next().and_then(|n| n.rsplit_once('.')).map_or("", |x| x.1)),
        What::App | What::File => it.path.clone(),
        _ => return null_mut(),
    };
    for (k, ic) in std::mem::take(&mut *DONE.lock().unwrap()) {
        remember_icon(ic as HICON);
        ICONS.with_borrow_mut(|m| m.insert(k, ic as HICON));
    }
    let key = wide(&key);
    if let Some(ic) = ICONS.with_borrow(|m| m.get(&key).copied()) {
        return ic;
    }
    if WAITING.with_borrow_mut(|w| w.insert(key.clone())) {
        let mut ask = ASK.lock().unwrap();
        let tx = ask.get_or_insert_with(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            thread::spawn(move || unsafe { load_icons(rx) });
            tx
        });
        let _ = tx.send((key, it.what, it.path.clone(), FH.get() as isize));
    }
    null_mut()
}

pub(super) unsafe fn paint(dc: HDC, f: &Fonts, over: Option<Act>) {
    let hot = |a: Act| over == Some(a);
    let bg = pal().bg;
    let q = QUERY.with_borrow(|q| q.clone());
    // search field
    let t = rise(0);
    let field = drop_by(r(16, 16, W - 16, 58), t);
    fill(dc, field, mix(bg, pal().text, 0.08 * t), sc(24));
    SelectObject(dc, f.glyph);
    sym(dc, "\u{E721}", drop_by(r(24, 16, 56, 58), t), mix(bg, pal().dim, t));
    SelectObject(dc, f.txt);
    let line = drop_by(r(62, 16, W - 64, 58), t);
    if q.is_empty() {
        text(dc, t!("Приложения, файлы, параметры, интернет", "Search apps, files, settings and the web", ja: "アプリ、ファイル、設定、ウェブを検索", pt: "Pesquisar apps, arquivos, configurações e web", zh: "搜索应用、文件、设置和网页"), line, mix(bg, pal().dim, t), DT_LEFT);
    } else {
        text(dc, &q, line, if SELALL.get() { pal().accent } else { pal().text }, DT_LEFT);
    }
    // a caret that breathes: the panel takes the keys at once (timer 1 repaints every second)
    if now_ms() / 1000 % 2 == 0 {
        let x = line.left + if q.is_empty() { -sc(4) } else { text_w(dc, &q).min(line.right - line.left) + sc(2) };
        fill(dc, RECT { left: x, right: x + sc(1).max(1), top: line.top + sc(12), bottom: line.bottom - sc(12) }, pal().accent, 0);
    }
    let rs = rects();
    let list = items();
    if !searching() {
        let t = rise(1);
        SelectObject(dc, f.semi);
        let title = if ALL.get() { t!("Все приложения", "All apps", ja: "すべてのアプリ", pt: "Todos os apps", zh: "所有应用") } else if PINNED.lock().unwrap().is_empty() { t!("Часто используемые", "Most used", ja: "よく使うアプリ", pt: "Mais usados", zh: "最常用") } else { t!("Закреплённые", "Pinned", ja: "ピン留め済み", pt: "Fixados", zh: "已固定") };
        text(dc, title, drop_by(r(28, 72, W - 180, 102), t), mix(bg, pal().text, t), DT_LEFT);
        if let Some(&(b, a)) = rs.iter().find(|(_, a)| *a == Act::AllApps) {
            let b = drop_by(b, t);
            let h = hv(1_800_001, hot(a));
            fill(dc, b, mix(bg, pal().text, (0.07 + 0.05 * h) * t), sc(20));
            SelectObject(dc, f.small);
            text(dc, if ALL.get() { t!("‹  Назад", "‹  Back", ja: "‹  戻る", pt: "‹  Voltar", zh: "‹  返回") } else { t!("Все приложения  ›", "All apps  ›", ja: "すべてのアプリ  ›", pt: "Todos os apps  ›", zh: "所有应用  ›") }, b, mix(bg, pal().text, 0.9 * t), DT_CENTER);
        }
        if list.is_empty() {
            SelectObject(dc, f.txt);
            text(dc, t!("Загрузка приложений…", "Loading apps…", ja: "アプリを読み込み中…", pt: "Carregando apps…", zh: "正在加载应用…"), r(28, TOP, W - 28, TOP + 40), pal().dim, DT_LEFT);
        }
    }
    for (rc, a) in &rs {
        let Act::Hit(i) = *a else { continue };
        let Some(it) = list.get(i) else { continue };
        let n = if searching() { i - SCROLL.get() } else { (i / COLS).saturating_sub(SCROLL.get()) * 2 + i % COLS };
        let t = rise(2 + (n as i32).min(10));
        let seen = |c: u32| mix(bg, c, t);
        let rc = drop_by(*rc, t);
        let key = 1_810_000 + n as i32;
        let (th, p) = (hv(key, hot(*a)), press_t(key));
        let sel = i == SEL.get() && (searching() || SEL_SHOWN.get());
        let base = mix(bg, pal().text, (0.08 * th + 0.05 * p).max(if sel { 0.10 } else { 0.0 }));
        if base != bg {
            fill(dc, rc, seen(base), nested(Kind::Start, 12, 8));
        }
        let ic = icon(it);
        let glyph = match it.what { What::Setting => "\u{E713}", What::Web => "\u{E774}", What::Sum => "\u{E8EF}", What::Folder => "\u{E8B7}", _ => "\u{E8A5}" };
        if searching() {
            let cy = (rc.top + rc.bottom) / 2;
            if ic.is_null() {
                SelectObject(dc, f.glyph);
                sym(dc, glyph, RECT { left: rc.left + sc(10), right: rc.left + sc(42), ..rc }, seen(if sel { pal().accent } else { pal().dim }));
            } else {
                fly_icon(dc, ic, (rc.left + sc(26)) as f32, cy as f32, sc(24) as f32 * (0.7 + 0.3 * t) * (1.0 - 0.08 * p));
            }
            let right = rc.right - sc(110);
            let left = rc.left + sc(52);
            SelectObject(dc, f.semi);
            let name = if it.what == What::Sum { format!("= {}", it.name) } else { it.name.clone() };
            if it.sub.is_empty() || it.what == What::Setting {
                text(dc, &name, RECT { left, right, ..rc }, seen(pal().text), DT_LEFT);
            } else {
                text(dc, &name, RECT { left, right, top: cy - sc(19), bottom: cy + sc(1) }, seen(pal().text), DT_LEFT);
                SelectObject(dc, f.small);
                SetTextColor(dc, seen(pal().dim));
                let w = wide(&it.sub);
                let mut b = RECT { left, right, top: cy, bottom: cy + sc(18) };
                DrawTextW(dc, w.as_ptr(), -1, &mut b, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_PATH_ELLIPSIS); // a path loses its middle, not its end
            }
            SelectObject(dc, f.small);
            let kind = match it.what {
                _ if sel => "Enter ↵",
                What::App => t!("Приложение", "App", ja: "アプリ", pt: "App", zh: "应用"),
                What::Setting => t!("Параметры", "Settings", ja: "設定", pt: "Configurações", zh: "设置"),
                What::File => t!("Файл", "File", ja: "ファイル", pt: "Arquivo", zh: "文件"),
                What::Folder => t!("Папка", "Folder", ja: "フォルダー", pt: "Pasta", zh: "文件夹"),
                What::Sum => t!("Калькулятор", "Calculator", ja: "電卓", pt: "Calculadora", zh: "计算器"),
                What::Web => "Web",
            };
            text(dc, kind, RECT { left: right, right: rc.right - sc(14), ..rc }, seen(pal().dim), DT_RIGHT);
        } else {
            let cx = (rc.left + rc.right) / 2;
            if ic.is_null() {
                SelectObject(dc, f.glyph);
                sym(dc, glyph, RECT { top: rc.top + sc(14), bottom: rc.top + sc(54), ..rc }, seen(pal().dim));
            } else {
                fly_icon(dc, ic, cx as f32, (rc.top + sc(31)) as f32, sc(34) as f32 * (0.7 + 0.3 * t) * (1.0 - 0.08 * p));
            }
            SelectObject(dc, f.small);
            // up to two lines, centered under the icon
            SetTextColor(dc, seen(pal().text));
            let mut b = RECT { left: rc.left + sc(3), right: rc.right - sc(3), top: rc.top + sc(57), bottom: rc.top + sc(90) };
            DrawTextW(dc, wide(&it.name).as_ptr(), -1, &mut b, DT_CENTER | DT_WORDBREAK | DT_EDITCONTROL | DT_END_ELLIPSIS | DT_NOPREFIX);
        }
    }
    // where the list is, when it does not fit
    let (n, shown) = lines(list.len());
    if n > shown {
        let (a, b) = if searching() { (LIST_TOP, LIST_TOP + ROW * LIST_ROWS as i32) } else { (TOP, TOP + CELL.1 * GRID_ROWS as i32) };
        let (a, b) = (sc(a), sc(b));
        let len = ((b - a) * shown as i32 / n as i32).max(sc(24));
        let y = a + (b - a - len) * SCROLL.get() as i32 / (n - shown) as i32;
        fill(dc, RECT { left: sc(W - 7), right: sc(W - 4), top: y, bottom: y + len }, mix(bg, pal().text, 0.25), sc(3));
    }
    // footer
    let t = rise(3);
    let seen = |c: u32| mix(bg, c, t);
    fill(dc, RECT { left: sc(20), right: sc(W - 20), top: sc(H - 60), bottom: sc(H - 60) + 1 }, seen(pal().line), 0);
    let user = std::env::var("USERNAME").unwrap_or_default();
    let av = r(20, H - 46, 50, H - 16);
    fill(dc, av, seen(mix(bg, pal().accent, 0.28)), sc(30));
    SelectObject(dc, f.semi);
    text(dc, &user.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default(), av, seen(pal().text), DT_CENTER);
    SelectObject(dc, f.txt);
    let note = NOTE.with_borrow(|n| n.clone());
    text(dc, if note.is_empty() { &user } else { &note }, r(60, H - 50, W - 150, H - 12), seen(mix(pal().dim, pal().text, 0.4)), DT_LEFT);
    SelectObject(dc, f.glyph);
    for (a, g) in [(Act::Explorer, "\u{E8B7}"), (Act::Settings("ms-settings:"), "\u{E713}"), (Act::PowerMenu, "\u{E7E8}")] {
        let Some(&(b, _)) = rs.iter().find(|(_, x)| *x == a) else { continue };
        let h = hv(1_800_010 + b.left, hot(a) || (a == Act::PowerMenu && PMENU.get() != 0));
        if h > 0.01 {
            fill(dc, b, seen(mix(bg, pal().text, 0.1 * h)), nested(Kind::Start, 12, 8));
        }
        sym_t(dc, g, b, seen(mix(mix(pal().dim, pal().text, 0.6), pal().text, h)), 0.0, press_t(1_800_010 + b.left));
    }
    if let Some(&(b, a)) = rs.iter().find(|(_, a)| *a == Act::Keyboard) {
        let h = hv(1_800_002, hot(a));
        if h > 0.01 {
            fill(dc, b, mix(bg, pal().text, 0.08 + 0.1 * h), sc(16));
        }
        SelectObject(dc, f.glyph);
        sym(dc, "\u{E765}", drop_by(b, rise(0)), mix(bg, mix(pal().dim, pal().text, h), rise(0)));
    }
    // the power menu grows out of its button
    if PMENU.get() != 0 {
        let e = (now_ms().saturating_sub(PMENU.get()) as f32 / 180.0).min(1.0);
        if e < 1.0 {
            BUSY.set(true);
        }
        let e = 1.0 - (1.0 - e).powi(3);
        let full = power_rect();
        let box_ = RECT { top: full.bottom - ((full.bottom - full.top) as f32 * (0.6 + 0.4 * e)) as i32, ..full };
        ICO.with_borrow_mut(|v| v.retain(|i| i.1 + i.3 < box_.left as f32 || i.2 + i.3 < box_.top as f32 || i.2 - i.3 > box_.bottom as f32)); // icons go over the glass: not over this
        fill(dc, RECT { left: box_.left - 1, top: box_.top - 1, right: box_.right + 1, bottom: box_.bottom + 1 }, mix(bg, 0, 0.5), sc(26));
        fill(dc, box_, mix(bg, pal().text, 0.13), sc(26));
        for (b, a) in rs.iter().rev().filter(|(b, _)| b.top >= box_.top) {
            let Act::Power(p) = *a else { continue };
            let h = hv(1_800_030 + p as i32, hot(*a));
            if h > 0.01 {
                fill(dc, *b, mix(bg, if p == 3 { BAD } else { pal().text }, 0.13 + 0.12 * h), sc(14));
            }
            SelectObject(dc, f.glyph);
            sym(dc, ["\u{E72E}", "\u{E708}", "\u{E777}", "\u{E7E8}"][p as usize], RECT { left: b.left + sc(6), right: b.left + sc(38), ..*b }, mix(bg, if p == 3 { mix(pal().text, BAD, 0.8) } else { pal().text }, e));
            SelectObject(dc, f.txt);
            let label = match p {
                0 => t!("Блокировка", "Lock", ja: "ロック", pt: "Bloquear", zh: "锁定"),
                1 => t!("Сон", "Sleep", ja: "スリープ", pt: "Suspender", zh: "睡眠"),
                2 => t!("Перезагрузка", "Restart", ja: "再起動", pt: "Reiniciar", zh: "重启"),
                _ => t!("Выключение", "Shut down", ja: "シャットダウン", pt: "Desligar", zh: "关机"),
            };
            text(dc, label, RECT { left: b.left + sc(46), ..*b }, mix(bg, pal().text, e), DT_LEFT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums() {
        assert_eq!(sum("2+2"), Some(4.0));
        assert_eq!(sum("12,5*4"), Some(50.0));
        assert_eq!(sum("(2+3)^2 - 1"), Some(24.0));
        assert_eq!(sum("10 / 4"), Some(2.5));
        assert_eq!(sum("-3+5"), Some(2.0));
        for no in ["chrome", "7", "1/0", "2+", "(2+3", "c++", "h-2", ""] {
            assert_eq!(sum(no), None, "{no}");
        }
        assert_eq!(sum(&"(".repeat(5000)), None);
        assert_eq!(sum(&("1+".repeat(300) + "1")), Some(301.0));
    }

    #[test]
    fn accents_do_not_matter() {
        assert_eq!(fold("Configurações É"), "configuracoes e");
        let hit = |q: &str| SETTINGS.iter().filter_map(|s| Some((setting_score(s, &fold(q))?, s.3))).min().map(|x| x.1);
        assert_eq!(hit("notificacoes"), Some("ms-settings:notifications"));
        assert_eq!(hit("papel"), Some("ms-settings:personalization-background"));
        assert_eq!(hit("update"), Some("ms-settings:windowsupdate"));
        assert_eq!(hit("zzzz"), None);
    }

    #[test]
    fn the_users_own_files_come_first() {
        let home = r"C:\Users\Ana";
        let s = |p: &str| file_score(p, "relatorio", home);
        assert!(s(r"C:\Users\Ana\Documents\Relatório.docx") > s(r"C:\Users\Ana\AppData\Local\x\relatorio.docx"));
        assert!(s(r"C:\Users\Ana\Downloads\relatorio.pdf") > s(r"C:\Windows\relatorio.pdf"));
        assert!(s(r"C:\Users\Ana\Desktop\relatorio.pdf") > s(r"C:\Users\Ana\Desktop\meu relatorio.pdf"));
        assert!(s(r"C:\Users\Ana\Desktop\relatorio.pdf") > s(r"C:\Users\Beto\Desktop\relatorio.pdf"));
        assert_eq!(s(r"C:\$Recycle.Bin\S-1\relatorio.pdf"), None);
        assert_eq!(s(r"C:\Users\Ana\Desktop\x\.git\refs\relatorio"), None);
        assert_eq!(s(r"C:\Users\Ana\Downloads\.~lock.relatorio.csv#"), None);
    }

    #[test]
    fn web_addresses_are_escaped() {
        assert_eq!(percent("a b&c=é"), "a%20b%26c%3D%C3%A9");
    }
}
