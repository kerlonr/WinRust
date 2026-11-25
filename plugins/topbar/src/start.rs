//! The Start panel of the Delphos style: a grid of apps, and one search over apps, settings, files (the rust-search index),
//! sums and the web. (A child of fly.rs: it shares its items through `super::*`.)
use super::*;
use std::sync::atomic::AtomicU32;

pub(super) const W: i32 = 600;
pub(super) const H: i32 = 476;
const COLS: usize = 6;
const GRID_ROWS: usize = 3;
const CELL: (i32, i32) = (92, 96);
const TOP: i32 = 112; // where the grid starts
const LIST_TOP: i32 = 76;
const ROW: i32 = 46;
const LIST_ROWS: usize = 7;
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
    pub(super) static CHOOSE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) }; // choosing what Start shows: what was in it when that began (listed first)
    static DRAGGED: RefCell<Option<(String, i32, i32)>> = const { RefCell::new(None) }; // the pinned app being dragged (its path) and the pointer
    static POS: RefCell<HashMap<String, (f32, f32)>> = RefCell::new(HashMap::new()); // where each app of the grid is drawn while they change places
    static MOVED: Cell<u64> = const { Cell::new(0) }; // when a drag last moved something
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

/// The results list is up (while choosing apps, typing narrows the grid instead).
pub(super) fn searching() -> bool {
    !choosing() && !typed().is_empty()
}

pub(super) fn choosing() -> bool {
    CHOOSE.with_borrow(|c| c.is_some())
}

/// The grid shows the user's own choice, in the user's own order: what can be dragged around.
fn arranging() -> bool {
    !choosing() && !ALL.get() && !searching()
}

fn app_item(a: &(String, Vec<u16>)) -> Item {
    Item { what: What::App, name: a.0.clone(), sub: String::new(), path: wstr(&a.1) }
}

/// What the panel shows now, in order.
pub(super) fn items() -> Vec<Item> {
    let raw = typed();
    if choosing() {
        return choose_items(&fold(&raw));
    }
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

/// The same path, letter case aside.
fn same(w: &[u16], p: &[u16]) -> bool {
    let low = |c: u16| if (65..91).contains(&c) { c + 32 } else { c };
    w.len() == p.len() && w.iter().zip(p).all(|(&a, &b)| low(a) == low(b))
}

/// Help files, uninstallers and the like, which installers leave among the apps: search finds them, the list of all apps leaves them out.
pub(super) fn clutter(name: &str) -> bool {
    let n = fold(name);
    let words: Vec<&str> = n.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    ["help", "ajuda", "changelog", "readme", "uninstall", "uninstaller", "desinstalar", "desinstalador", "manual", "documentation", "documentacao", "license", "licenca", "website"].iter().any(|w| words.contains(w)) || n.contains("release notes")
}

fn listed(a: &(String, Vec<u16>)) -> bool {
    a.1.starts_with(&[115, 104, 101, 108, 108, 58]) || !clutter(&a.0) // "shell:": a Store app
}

const DEFAULTS: usize = 12; // two rows

/// What Start shows until the user chooses: what they launch most, then the everyday apps of Windows.
pub(super) fn defaults(apps: &[(String, Vec<u16>)]) -> Vec<String> {
    const EVERYDAY: [&[&str]; 8] = [&["explorador de arquivos", "file explorer"], &["google chrome"], &["microsoft edge"], &["configuracoes", "settings"], &["terminal"], &["bloco de notas", "notepad"], &["calculadora", "calculator"], &["fotos", "photos"]];
    let mut v: Vec<(u32, usize)> = apps.iter().enumerate().filter(|(_, a)| listed(a)).map(|(i, a)| (u32::MAX - uses(&a.0.to_lowercase()), i)).filter(|x| x.0 != u32::MAX).collect();
    v.sort_unstable();
    let mut v: Vec<usize> = v.into_iter().map(|x| x.1).take(DEFAULTS).collect();
    for names in EVERYDAY {
        if let Some(i) = apps.iter().position(|a| names.contains(&fold(&a.0).as_str()))
            && v.len() < DEFAULTS
            && !v.contains(&i)
        {
            v.push(i);
        }
    }
    v.into_iter().map(|i| wstr(&apps[i].1)).collect()
}

/// What Start shows now: the user's choice, or the defaults.
pub(super) fn current(apps: &[(String, Vec<u16>)]) -> Vec<String> {
    if CHOSEN.load(Relaxed) { PINNED.lock().unwrap().clone() } else { defaults(apps) }
}

/// Every app, what was in Start first.
fn choose_items(q: &str) -> Vec<Item> {
    let first: Vec<Vec<u16>> = CHOOSE.with_borrow(|c| c.as_ref().map(|c| c.iter().map(|p| wide(p)).collect()).unwrap_or_default());
    let apps = APPS.lock().unwrap();
    let mut v: Vec<Item> = first.iter().filter_map(|p| apps.iter().find(|a| same(&a.1, p)).map(app_item).or_else(|| loose(&wstr(p)))).collect();
    v.extend(apps.iter().filter(|a| listed(a) && !first.iter().any(|f| same(&a.1, f))).map(app_item));
    if !q.is_empty() {
        v.retain(|i| score(&fold(&i.name), q).is_some());
    }
    v
}

/// A pinned file that is not one of the apps. None = it is gone.
fn loose(p: &str) -> Option<Item> {
    if p.to_lowercase().starts_with("shell:") || GONE.lock().unwrap().iter().any(|g| g == p) {
        return None;
    }
    Some(Item { what: What::App, name: std::path::Path::new(p).file_stem().unwrap_or_default().to_string_lossy().into(), sub: String::new(), path: p.into() })
}

fn grid_items() -> Vec<Item> {
    let apps = APPS.lock().unwrap();
    if ALL.get() {
        return apps.iter().filter(|a| listed(a)).map(app_item).collect();
    }
    current(&apps)
        .iter()
        .filter_map(|p| {
            let w = wide(p);
            apps.iter().find(|a| same(&a.1, &w)).map(app_item).or_else(|| loose(p))
        })
        .collect()
}

pub(super) fn begin_choosing() {
    let now = current(&APPS.lock().unwrap());
    CHOOSE.set(Some(now));
    QUERY.set(String::new());
    ALL.set(false);
    top();
}

pub(super) fn end_choosing() -> bool {
    let was = CHOOSE.take().is_some();
    if was {
        QUERY.set(String::new());
        top();
    }
    was
}

/// Another list: from its top, coming in like the first one did.
pub(super) fn top() {
    SCROLL.set(0);
    SEL.set(0);
    SEL_SHOWN.set(false);
    MOVED.set(0);
    T0.set(unsafe { now_ms() });
}

// ---------- changing places ----------

/// Dragging item `i` to (x, y): the app under the pointer makes room.
pub(super) fn drag(i: usize, x: i32, y: i32) {
    if !arranging() {
        return;
    }
    if DRAGGED.with_borrow(|d| d.is_none()) {
        let Some(it) = items().into_iter().nth(i) else { return };
        chosen();
        DRAGGED.set(Some((it.path, x, y)));
    }
    let path = DRAGGED.with_borrow_mut(|d| {
        let d = d.as_mut().unwrap();
        (d.1, d.2) = (x, y);
        d.0.clone()
    });
    let list = items();
    let Some(from) = list.iter().position(|it| it.path.eq_ignore_ascii_case(&path)) else { return };
    if let Some((_, Act::Hit(to))) = act_at(Kind::Start, x, y)
        && to != from
        && to < list.len()
    {
        move_pin(&path, &list[to].path);
    }
}

/// Puts `path` where `at` is; what is between moves over.
pub(super) fn move_pin(path: &str, at: &str) {
    let mut p = PINNED.lock().unwrap();
    let find = |x: &str| p.iter().position(|y| y.eq_ignore_ascii_case(x));
    if let (Some(a), Some(b)) = (find(path), find(at)) {
        let x = p.remove(a);
        p.insert(b, x);
    }
}

/// The drag is over (`keep`: where it was dropped counts). false = there was none.
pub(super) fn drop_drag(keep: bool) -> bool {
    if DRAGGED.take().is_none() {
        return false;
    }
    MOVED.set(unsafe { now_ms() });
    if keep { pinned_save() } else { pinned_load() }
    true
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
    let step = if searching() || n > 6 { 2 } else { 1 };
    SCROLL.set(if up { SCROLL.get().saturating_sub(step) } else { (SCROLL.get() + step).min(n.saturating_sub(shown)) });
}

fn button(i: i32) -> RECT {
    // the footer's buttons: Explorer (2) and settings (1) at the left, power (0) at the right
    let x = if i == 0 { W - 50 } else { 12 + 42 * (2 - i) };
    r(x, H - 50, x + 38, H - 12)
}

fn power_rect() -> RECT {
    r(W - 12 - PW, H - 58 - 12 - 4 * PROW, W - 12, H - 58)
}

/// Animation key of item `i`'s cell: one of its own for each cell in sight.
pub(super) fn cell_key(i: usize) -> i32 {
    1_810_000 + i as i32 - (SCROLL.get() * if searching() { 1 } else { COLS }) as i32
}

/// The same for a button of the footer.
pub(super) fn button_key(a: Act) -> i32 {
    match a {
        Act::Explorer => 1_800_010,
        Act::Settings(_) => 1_800_011,
        _ => 1_800_012,
    }
}

/// Is (x, y) on the open power menu (between its items too)? What is under it is out of reach.
pub(super) fn on_power(x: i32, y: i32) -> bool {
    let p = power_rect();
    PMENU.get() != 0 && x >= p.left && x < p.right && y >= p.top && y < p.bottom
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
        if choosing() {
            v.push((r(W - 134, 72, W - 24, 102), Act::Choose));
        } else {
            v.push((r(W - 154, 72, W - 24, 102), Act::AllApps));
            v.push((r(W - 240, 72, W - 162, 102), Act::Choose));
        }
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
    if choosing() {
        start_toggle(&it.path);
        return false; // stays open
    }
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
    if choosing() {
        return; // a click is all there is to it
    }
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
    if arranging() && i > 0 {
        add(5, t!("Переместить в начало", "Move to front", ja: "先頭に移動", pt: "Mover para o início", zh: "移到最前"));
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
        5 => {
            chosen();
            if let Some(first) = items().first() {
                move_pin(&it.path, &first.path);
                pinned_save();
            }
            return;
        }
        _ => return,
    }
    close();
}

// ---------- painting ----------

const INK: u32 = 0x1A_1210; // what is written on the accent colour

pub(super) struct Fonts {
    pub txt: HFONT,
    pub small: HFONT,
    pub glyph: HFONT,
    pub semi: HFONT,
    pub mark: HFONT,
}

type Ask = (Vec<u16>, What, String, isize); // key, what it is, path, the panel to repaint
static ASK: Mutex<Option<std::sync::mpsc::Sender<Ask>>> = Mutex::new(None);
static DONE: Mutex<Vec<(Vec<u16>, isize)>> = Mutex::new(Vec::new()); // loaded icons on their way back
const LOADERS: usize = 4;

fn cache_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join(r"WinRust\icons")
}

/// Where the picture of `path`'s icon is kept: another file once the shortcut changes or the icons get another size.
fn cache_file(path: &str, px: i32) -> PathBuf {
    let stamp = if path.contains(":\\") { std::fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs()) } else { 0 };
    let h = format!("{}|{stamp}|{px}", path.to_lowercase()).bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    cache_dir().join(format!("{h:016x}.ico32"))
}

/// A kept picture (its size, then size x size pixels) as an icon; null when there is none.
unsafe fn cache_read(f: &std::path::Path) -> HICON {
    let Ok(d) = std::fs::read(f) else { return null_mut() };
    let px = d.get(..4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap())) as usize;
    if !(8..=256).contains(&px) || d.len() != 4 + px * px * 4 {
        return null_mut(); // half written, or not ours
    }
    let bits: Vec<u32> = d[4..].chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    let px = px as i32;
    let (color, mask) = (CreateBitmap(px, px, 1, 32, bits.as_ptr() as *const _), CreateBitmap(px, px, 1, 1, null()));
    let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
    let ic = CreateIconIndirect(&ii);
    DeleteObject(mask);
    DeleteObject(color);
    ic
}

unsafe fn cache_write(f: &std::path::Path, ic: HICON) {
    let mut ii: ICONINFO = zeroed();
    if ic.is_null() || GetIconInfo(ic, &mut ii) == 0 {
        return;
    }
    let mut b: BITMAP = zeroed();
    if !ii.hbmColor.is_null() && GetObjectW(ii.hbmColor, size_of::<BITMAP>() as i32, &mut b as *mut _ as *mut _) != 0 && b.bmWidth == b.bmHeight && (8..=256).contains(&b.bmWidth) {
        let px = b.bmWidth;
        let mut bi: BITMAPINFO = zeroed();
        bi.bmiHeader = BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: px, biHeight: -px, biPlanes: 1, biBitCount: 32, ..zeroed() };
        let mut bits = vec![0u32; (px * px) as usize];
        let dc = GetDC(null_mut());
        let got = GetDIBits(dc, ii.hbmColor, 0, px as u32, bits.as_mut_ptr() as *mut _, &mut bi, DIB_RGB_COLORS);
        ReleaseDC(null_mut(), dc);
        // without alpha the picture needs its mask: those are not kept
        if got == px && bits.iter().any(|p| p >> 24 != 0) {
            let mut d = (px as u32).to_le_bytes().to_vec();
            d.extend(bits.iter().flat_map(|p| p.to_le_bytes()));
            // in one step: another reader never sees half of it
            let tmp = f.with_extension(format!("{:?}.tmp", thread::current().id()).replace(['(', ')'], ""));
            if std::fs::write(&tmp, d).is_ok() && std::fs::rename(&tmp, f).is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
    DeleteObject(ii.hbmColor);
    DeleteObject(ii.hbmMask);
}

/// An app's icon from the shell (slow the first time after Windows starts), kept for next time.
unsafe fn app_fresh(path: &str, f: &std::path::Path) -> HICON {
    let w = wide(path);
    let mut ic = null_mut();
    for n in 0..3 {
        // the shell sometimes has no answer the first time
        if n > 0 {
            thread::sleep(std::time::Duration::from_millis(60));
        }
        ic = lnk_icon(&w, &lnk_target(&w));
        if !ic.is_null() {
            break;
        }
    }
    cache_write(f, ic);
    ic
}

/// Loads icons on threads of their own: some take the shell a second, and neither typing nor the other icons must wait.
unsafe fn load_icons(rx: std::sync::Arc<Mutex<std::sync::mpsc::Receiver<Ask>>>, scale: f32) {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    SCALE.set(scale); // one per thread: load_px() follows it
    let px = load_px();
    let _ = std::fs::create_dir_all(cache_dir());
    loop {
        let next = rx.lock().unwrap().recv();
        let Ok((key, what, path, hw)) = next else { return };
        let mut ic;
        if what == What::App {
            let f = cache_file(&path, px);
            ic = cache_read(&f);
            if ic.is_null() {
                ic = app_fresh(&path, &f);
            }
        } else {
            ic = item_icon(&path, px);
        }
        if ic.is_null() && what == What::File {
            // not ours to read (another user's): what its kind of file looks like
            let mut sfi: SHFILEINFOW = zeroed();
            if SHGetFileInfoW(wide(&path).as_ptr(), 0x80, &mut sfi, size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES) != 0 {
                ic = sfi.hIcon;
            }
        }
        forget_icon(ic); // the smooth copy belongs to the thread that draws
        DONE.lock().unwrap().push((key, ic as isize));
        if hw != 0 {
            InvalidateRect(hw as HWND, null(), 0);
        }
    }
}

fn ask(a: Ask) {
    let mut g = ASK.lock().unwrap();
    let tx = g.get_or_insert_with(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        let (rx, scale) = (std::sync::Arc::new(Mutex::new(rx)), SCALE.get());
        for _ in 0..LOADERS {
            let rx = rx.clone();
            thread::spawn(move || unsafe { load_icons(rx, scale) });
        }
        tx
    });
    let _ = tx.send(a);
}

/// The icons Start opens with, before it is first opened.
pub(super) fn preload() {
    for it in grid_items() {
        let key = wide(&it.path);
        if WAITING.with_borrow_mut(|w| w.insert(key.clone())) {
            ask((key, What::App, it.path, 0));
        }
    }
}

/// Pictures of every app's icon, made quietly once the PC has settled and made again once a week: "All apps" opens with them.
pub(super) fn warm() {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN};
    let scale = SCALE.get();
    thread::spawn(move || unsafe {
        thread::sleep(std::time::Duration::from_secs(25));
        SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        SCALE.set(scale);
        let px = load_px();
        let _ = std::fs::create_dir_all(cache_dir());
        let days = |f: &std::path::Path| std::fs::metadata(f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_secs() / 86400);
        for e in std::fs::read_dir(cache_dir()).into_iter().flatten().flatten() {
            if days(&e.path()).is_some_and(|d| d > 60) {
                let _ = std::fs::remove_file(e.path()); // of an app long gone
            }
        }
        let apps: Vec<String> = APPS.lock().unwrap().iter().filter(|a| listed(a)).map(|a| wstr(&a.1)).collect();
        for p in apps {
            let f = cache_file(&p, px);
            if days(&f).is_none_or(|d| d >= 7) {
                let ic = app_fresh(&p, &f);
                forget_icon(ic);
                DestroyIcon(ic);
                thread::sleep(std::time::Duration::from_millis(30));
            }
        }
    });
}

/// Icons that had no answer get another chance the next time Start opens.
pub(super) fn retry() {
    ICONS.with_borrow_mut(|m| m.retain(|k, ic| !ic.is_null() || { WAITING.with_borrow_mut(|w| w.remove(k)); false }));
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
        ask((key, it.what, it.path.clone(), FH.get() as isize));
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
        let what = if choosing() { t!("Поиск приложений", "Search apps", ja: "アプリを検索", pt: "Pesquisar apps", zh: "搜索应用") } else { t!("Приложения, файлы, параметры, интернет", "Search apps, files, settings and the web", ja: "アプリ、ファイル、設定、ウェブを検索", pt: "Pesquisar apps, arquivos, configurações e web", zh: "搜索应用、文件、设置和网页") };
        text(dc, what, line, mix(bg, pal().dim, t), DT_LEFT);
    } else {
        text(dc, &q, line, if SELALL.get() { pal().accent } else { pal().text }, DT_LEFT);
    }
    // a caret that breathes: the panel takes the keys at once (timer 1 repaints every second)
    if now_ms() / 1000 % 2 == 0 {
        let x = line.left + if q.is_empty() { -sc(4) } else { text_w(dc, &q).min(line.right - line.left) + sc(2) };
        fill(dc, RECT { left: x, right: x + sc(1).max(1), top: line.top + sc(12), bottom: line.bottom - sc(12) }, pal().accent, 0);
    }
    let list = items();
    let (n, shown) = lines(list.len());
    SCROLL.set(SCROLL.get().min(n.saturating_sub(shown))); // something left the list
    let rs = rects();
    if !searching() {
        let t = rise(1);
        SelectObject(dc, f.semi);
        let title = if choosing() { t!("Выберите приложения для Пуска", "Choose what Start shows", ja: "スタートに表示するアプリ", pt: "Escolha o que aparece no Iniciar", zh: "选择开始菜单中的应用") } else if ALL.get() { t!("Все приложения", "All apps", ja: "すべてのアプリ", pt: "Todos os apps", zh: "所有应用") } else if !CHOSEN.load(Relaxed) { t!("Часто используемые", "Most used", ja: "よく使うアプリ", pt: "Mais usados", zh: "最常用") } else { t!("Закреплённые", "Pinned", ja: "ピン留め済み", pt: "Fixados", zh: "已固定") };
        text(dc, title, drop_by(r(28, 72, if choosing() { W - 150 } else { W - 260 }, 102), t), mix(bg, pal().text, t), DT_LEFT);
        if let Some(&(b, a)) = rs.iter().find(|(_, a)| *a == Act::AllApps) {
            let b = drop_by(b, t);
            let h = hv(1_800_001, hot(a));
            fill(dc, b, mix(bg, pal().text, (0.07 + 0.05 * h) * t), sc(20));
            SelectObject(dc, f.small);
            text(dc, if ALL.get() { t!("‹  Назад", "‹  Back", ja: "‹  戻る", pt: "‹  Voltar", zh: "‹  返回") } else { t!("Все приложения  ›", "All apps  ›", ja: "すべてのアプリ  ›", pt: "Todos os apps  ›", zh: "所有应用  ›") }, b, mix(bg, pal().text, 0.9 * t), DT_CENTER);
        }
        if let Some(&(b, a)) = rs.iter().find(|(_, a)| *a == Act::Choose) {
            let b = drop_by(b, t);
            let h = hv(1_800_003, hot(a));
            let on = choosing();
            fill(dc, b, if on { mix(bg, pal().accent, (0.85 + 0.15 * h) * t) } else { mix(bg, pal().text, (0.07 + 0.05 * h) * t) }, sc(20));
            SelectObject(dc, if on { f.semi } else { f.small });
            text(dc, if on { t!("Готово", "Done", ja: "完了", pt: "Concluído", zh: "完成") } else { t!("Изменить", "Edit", ja: "編集", pt: "Editar", zh: "编辑") }, b, if on { mix(bg, INK, t) } else { mix(bg, pal().text, 0.9 * t) }, DT_CENTER);
        }
        if list.is_empty() {
            SelectObject(dc, f.txt);
            let why = if choosing() && !q.is_empty() {
                t!("Приложения не найдены", "No apps found", ja: "アプリが見つかりません", pt: "Nenhum app encontrado", zh: "未找到应用")
            } else if arranging() && CHOSEN.load(Relaxed) {
                t!("Здесь пусто: нажмите «Изменить», чтобы выбрать приложения", "Nothing here yet: press Edit to choose the apps", ja: "「編集」でアプリを選んでください", pt: "Nada aqui ainda. Clique em Editar para escolher os apps", zh: "这里是空的：点“编辑”选择应用")
            } else {
                t!("Загрузка приложений…", "Loading apps…", ja: "アプリを読み込み中…", pt: "Carregando apps…", zh: "正在加载应用…")
            };
            text(dc, why, r(28, TOP, W - 28, TOP + 40), pal().dim, DT_LEFT);
        }
    }
    let held = DRAGGED.with_borrow(|d| d.clone());
    let gliding = held.is_some() || now_ms().saturating_sub(MOVED.get()) < 250;
    if !gliding {
        POS.with_borrow_mut(|p| p.clear());
    }
    let picked = if choosing() { current(&APPS.lock().unwrap()) } else { vec![] };
    // the one being dragged last: over the others
    let is_held = |a: &Act| matches!(a, Act::Hit(i) if held.as_ref().is_some_and(|h| list.get(*i).is_some_and(|it| it.path.eq_ignore_ascii_case(&h.0))));
    for (rc, a) in rs.iter().filter(|x| !is_held(&x.1)).chain(rs.iter().filter(|x| is_held(&x.1))) {
        let Act::Hit(i) = *a else { continue };
        let Some(it) = list.get(i) else { continue };
        let (mut rc, mine) = (*rc, is_held(a));
        if gliding {
            let to = (rc.left as f32, rc.top as f32);
            let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
            let at = POS.with_borrow_mut(|p| {
                let e = p.entry(it.path.clone()).or_insert(to);
                *e = match &held {
                    Some((_, x, y)) if mine => ((x - w / 2) as f32, (y - h / 2) as f32),
                    _ => (e.0 + (to.0 - e.0) * 0.3, e.1 + (to.1 - e.1) * 0.3),
                };
                (e.0.round() as i32, e.1.round() as i32)
            });
            rc = RECT { left: at.0, top: at.1, right: at.0 + w, bottom: at.1 + h };
            BUSY.set(true);
        }
        let n = if searching() { i - SCROLL.get() } else { (i / COLS).saturating_sub(SCROLL.get()) * 2 + i % COLS };
        let t = rise(2 + (n as i32).min(10));
        let seen = |c: u32| mix(bg, c, t);
        let rc = drop_by(rc, t);
        let key = cell_key(i);
        let (th, p) = (hv(key, hot(*a)), press_t(key));
        let sel = i == SEL.get() && (searching() || SEL_SHOWN.get());
        let base = mix(bg, pal().text, (0.08 * th + 0.05 * p).max(if mine { 0.14 } else if sel { 0.10 } else { 0.0 }));
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
            if choosing() {
                // in Start: a tick; not: an empty ring
                let on = picked.iter().any(|p| p.eq_ignore_ascii_case(&it.path));
                let m = glide(1_820_000 + i as i32, if on { 1.0 } else { 0.0 });
                let b = RECT { left: rc.right - sc(24), top: rc.top + sc(4), right: rc.right - sc(4), bottom: rc.top + sc(24) };
                fill(dc, b, seen(mix(mix(bg, pal().text, 0.35), pal().accent, m)), sc(20));
                let d = sc(1).max(1) + 1;
                fill(dc, inset(b, d), seen(mix(mix(bg, pal().text, 0.06), pal().accent, m)), sc(20) - 2 * d);
                if m > 0.3 {
                    SelectObject(dc, f.mark);
                    text(dc, "\u{E73E}", b, seen(INK), DT_CENTER);
                    text(dc, "\u{E73E}", RECT { left: b.left + 1, right: b.right + 1, ..b }, seen(INK), DT_CENTER); // thin at this size: twice
                }
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
    SelectObject(dc, f.small);
    let note = NOTE.with_borrow(|n| n.clone());
    let hint = if !note.is_empty() { &note } else if choosing() { t!("Нажмите на приложение, чтобы добавить или убрать", "Click an app to add or remove it", ja: "クリックで追加・削除", pt: "Clique em um app para adicionar ou remover", zh: "点击应用以添加或移除") } else { "" };
    text(dc, hint, r(110, H - 50, W - 110, H - 12), seen(pal().dim), DT_CENTER);
    SelectObject(dc, f.glyph);
    for (a, g) in [(Act::Explorer, "\u{E8B7}"), (Act::Settings("ms-settings:"), "\u{E713}"), (Act::PowerMenu, "\u{E7E8}")] {
        let Some(&(b, _)) = rs.iter().find(|(_, x)| *x == a) else { continue };
        let h = hv(button_key(a), hot(a) || (a == Act::PowerMenu && PMENU.get() != 0));
        if h > 0.01 {
            fill(dc, b, seen(mix(bg, pal().text, 0.1 * h)), nested(Kind::Start, 12, 8));
        }
        sym_t(dc, g, b, seen(mix(mix(pal().dim, pal().text, 0.6), pal().text, h)), 0.0, press_t(button_key(a)));
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

    /// `cargo test icon_speed -- --ignored --nocapture`: what filling the grid costs on this machine.
    #[test]
    #[ignore]
    fn icon_speed() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let t = std::time::Instant::now();
            load_apps();
            println!("apps: {} in {:?}", APPS.lock().unwrap().len(), t.elapsed());
            let a: Vec<_> = APPS.lock().unwrap().iter().cloned().collect();
            for n in 0..2 {
                let t = std::time::Instant::now();
                for x in a.iter().take(24) {
                    let u = std::time::Instant::now();
                    let k = lnk_target(&x.1);
                    let v = u.elapsed();
                    let ic = lnk_icon(&x.1, &k);
                    if n == 0 { println!("  {} target {:?} total {:?} {}", x.0, v, u.elapsed(), ic.is_null()); }
                }
                println!("24 icons: {:?}", t.elapsed());
            }
        }
    }

    #[test]
    fn kept_icons_come_back_the_same() {
        unsafe {
            let px = 48i32;
            let bits: Vec<u32> = (0..px * px).map(|i| if i % 3 == 0 { 0 } else { 0xFF00_0000 | i as u32 * 7 }).collect();
            let (c, m) = (CreateBitmap(px, px, 1, 32, bits.as_ptr() as *const _), CreateBitmap(px, px, 1, 1, null()));
            let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: m, hbmColor: c };
            let ic = CreateIconIndirect(&ii);
            let f = std::env::temp_dir().join(format!("topbar-test-{}.ico32", std::process::id()));
            cache_write(&f, ic);
            let first = std::fs::read(&f).unwrap();
            assert_eq!(first[8..12], bits[1].to_le_bytes());
            let back = cache_read(&f);
            assert!(!back.is_null());
            cache_write(&f, back);
            assert_eq!(first, std::fs::read(&f).unwrap());
            for bad in [&first[..100], &[], &[255; 40]] {
                std::fs::write(&f, bad).unwrap();
                assert!(cache_read(&f).is_null(), "{} bytes", bad.len());
            }
            let _ = std::fs::remove_file(&f);
            assert!(cache_read(&f).is_null(), "no file");
        }
    }

    #[test]
    fn clutter_is_left_out() {
        for yes in ["7-Zip Help", "Ajuda do WinRAR", "ChangeLog", "Git Release Notes", "Desinstalar X"] {
            assert!(clutter(yes), "{yes}");
        }
        for no in ["WinRAR", "Helper Studio", "Termius", "Manualidades"] {
            assert!(!clutter(no), "{no}");
        }
    }

    #[test]
    fn web_addresses_are_escaped() {
        assert_eq!(percent("a b&c=é"), "a%20b%26c%3D%C3%A9");
    }
}
