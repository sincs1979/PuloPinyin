use crate::candidate_window::CandidatePanel;
use crate::caps;
use engine::{Engine, KeyEvent, SessionOutput};
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, Bool};
use objc2::{define_class, msg_send, ClassType, DefinedClass, Message};
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags, NSEventType};
use objc2_foundation::{
    NSBundle, NSNotFound, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use objc2_input_method_kit::{IMKInputController, IMKServer};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[link(name = "System", kind = "dylib")]
extern "C" {
    static mut _dispatch_main_q: u8;
    fn dispatch_async_f(
        queue: *mut c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

/// Shared across every text field. `activateServer` must not reset this to Chinese.
static ASCII: AtomicBool = AtomicBool::new(false);
static UPPER: AtomicBool = AtomicBool::new(false);
static PRESS_GEN: AtomicU64 = AtomicU64::new(0);
static CAPS_HELD: AtomicBool = AtomicBool::new(false);
static DELAY_ON: AtomicBool = AtomicBool::new(false);
static LAST_CAPS_AT: AtomicU64 = AtomicU64::new(0);
static IGNORE_CAPS_UNTIL: AtomicU64 = AtomicU64::new(0);

struct LongPressJob {
    ctrl: usize,
    gen: u64,
}

unsafe extern "C" fn apply_long_press(ctx: *mut c_void) {
    let job = unsafe { Box::from_raw(ctx as *mut LongPressJob) };
    let this = unsafe { Retained::<BuluoInputController>::from_raw(job.ctrl as *mut _) };
    let Some(this) = this else {
        return;
    };
    if PRESS_GEN.load(Ordering::SeqCst) != job.gen {
        return;
    }
    this.set_ascii(true, true);
    caps::set_led(true);
    IGNORE_CAPS_UNTIL.store(
        caps::now_ms().saturating_add(LED_SWALLOW_MS),
        Ordering::SeqCst,
    );
    CAPS_HELD.store(false, Ordering::SeqCst);
    eprintln!("buluo: 长按，灯应亮");
}

const CAPS_LOCK_KEY: u16 = 57;
const EISU_KEY: u16 = 102;
const LEFT_SHIFT: u16 = 56;
const RIGHT_SHIFT: u16 = 60;
const LONG_PRESS_MS: u64 = 3000;
const SHIFT_TAP_MS: u64 = 400;
const SAME_PRESS_MS: u64 = 400;
const LED_SWALLOW_MS: u64 = 250;
const POLL_MS: u64 = 40;

pub struct ControllerIvars {
    engine: RefCell<Engine>,
    panel: RefCell<Option<CandidatePanel>>,
    last_caps: Cell<bool>,
    caps_upper: Cell<bool>,
    shift_down_at: Cell<u64>,
    shift_used: Cell<bool>,
}

define_class!(
    #[unsafe(super(IMKInputController))]
    #[name = "BuluoInputController"]
    #[ivars = ControllerIvars]
    struct BuluoInputController;

    unsafe impl NSObjectProtocol for BuluoInputController {}

    impl BuluoInputController {
        #[unsafe(method_id(initWithServer:delegate:client:))]
        fn init_with_server_delegate_client(
            this: Allocated<Self>,
            server: Option<&IMKServer>,
            delegate: Option<&AnyObject>,
            client: Option<&AnyObject>,
        ) -> Option<Retained<Self>> {
            let mut engine = open_engine();
            engine.set_ascii_mode(ASCII.load(Ordering::SeqCst));
            let this = this.set_ivars(ControllerIvars {
                engine: RefCell::new(engine),
                panel: RefCell::new(None),
                last_caps: Cell::new(
                    NSEvent::modifierFlags_class().contains(NSEventModifierFlags::CapsLock),
                ),
                caps_upper: Cell::new(UPPER.load(Ordering::SeqCst)),
                shift_down_at: Cell::new(0),
                shift_used: Cell::new(false),
            });
            unsafe {
                msg_send![super(this), initWithServer: server, delegate: delegate, client: client]
            }
        }

        #[unsafe(method(recognizedEvents:))]
        fn recognized_events(&self, _sender: Option<&AnyObject>) -> usize {
            (NSEventMask::KeyDown | NSEventMask::KeyUp | NSEventMask::FlagsChanged).bits() as usize
        }

        #[unsafe(method(handleEvent:client:))]
        fn handle_event(&self, event: Option<&NSEvent>, client: Option<&AnyObject>) -> Bool {
            match (event, client) {
                (Some(event), client) => {
                    if event.keyCode() == CAPS_LOCK_KEY || event.keyCode() == EISU_KEY {
                        return self.handle_cn_en_event(event).into();
                    }
                    match event.r#type() {
                        NSEventType::FlagsChanged => self.handle_flags(event).into(),
                        NSEventType::KeyUp => false.into(),
                        NSEventType::KeyDown => {
                            if event.modifierFlags().intersects(
                                NSEventModifierFlags::Command | NSEventModifierFlags::Control,
                            ) {
                                return false.into();
                            }
                            if let Some(client) = client {
                                self.handle_key(event, client).into()
                            } else {
                                false.into()
                            }
                        }
                        _ => false.into(),
                    }
                }
                _ => false.into(),
            }
        }

        #[unsafe(method(didCommandBySelector:client:))]
        fn did_command(
            &self,
            _selector: objc2::runtime::Sel,
            _client: Option<&AnyObject>,
        ) -> Bool {
            false.into()
        }

        #[unsafe(method(commitComposition:))]
        fn commit_composition(&self, sender: Option<&AnyObject>) {
            let mut engine = self.ivars().engine.borrow_mut();
            engine.end_ascii_run();
            if engine.composing().is_empty() {
                return;
            }
            let out = engine.handle_key(KeyEvent::Space);
            drop(engine);
            if let Some(client) = sender {
                apply_output(self, client, &out);
            }
        }

        #[unsafe(method(hidePalettes))]
        fn hide_palettes(&self) {
            if let Some(panel) = self.ivars().panel.borrow().as_ref() {
                panel.hide();
            }
        }

        #[unsafe(method(deactivateServer:))]
        fn deactivate_server(&self, sender: Option<&AnyObject>) {
            if let Some(client) = sender {
                let mut engine = self.ivars().engine.borrow_mut();
                engine.end_ascii_run();
                if !engine.composing().is_empty() {
                    let out = engine.handle_key(KeyEvent::Enter);
                    drop(engine);
                    apply_output(self, client, &out);
                }
            } else {
                self.ivars().engine.borrow_mut().end_ascii_run();
            }
            if let Some(panel) = self.ivars().panel.borrow().as_ref() {
                panel.hide();
            }
        }

        #[unsafe(method(activateServer:))]
        fn activate_server(&self, _sender: Option<&AnyObject>) {
            self.ivars().engine.borrow_mut().end_ascii_run();
            self.sync_mode();
            if DELAY_ON
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                caps::ensure_immediate_caps();
            }
            if UPPER.load(Ordering::SeqCst) {
                caps::set_led(true);
            }
        }
    }
);

impl BuluoInputController {
    fn hide_panel(&self) {
        if let Some(panel) = self.ivars().panel.borrow().as_ref() {
            panel.hide();
        }
    }

    fn sync_mode(&self) {
        let ascii = ASCII.load(Ordering::SeqCst);
        let upper = UPPER.load(Ordering::SeqCst);
        self.ivars().engine.borrow_mut().set_ascii_mode(ascii);
        self.ivars().caps_upper.set(upper);
    }

    fn set_ascii(&self, ascii: bool, upper: bool) {
        ASCII.store(ascii, Ordering::SeqCst);
        UPPER.store(upper, Ordering::SeqCst);
        let mut engine = self.ivars().engine.borrow_mut();
        if !engine.composing().is_empty() {
            let _ = engine.handle_key(KeyEvent::Escape);
        }
        engine.set_ascii_mode(ascii);
        drop(engine);
        self.ivars().caps_upper.set(upper);
        self.hide_panel();
        eprintln!(
            "buluo: {} {}",
            if ascii { "英文" } else { "中文" },
            if upper { "大写" } else { "小写" }
        );
    }

    fn toggle_language(&self) {
        if UPPER.load(Ordering::SeqCst) {
            self.set_ascii(false, false);
        } else {
            self.set_ascii(!ASCII.load(Ordering::SeqCst), false);
        }
    }

    fn handle_cn_en_event(&self, event: &NSEvent) -> bool {
        match event.r#type() {
            NSEventType::KeyUp => {
                self.finish_cn_en_tap();
                true
            }
            _ => {
                self.begin_cn_en();
                true
            }
        }
    }

    fn swallow_caps_led(&self) {
        IGNORE_CAPS_UNTIL.store(
            caps::now_ms().saturating_add(LED_SWALLOW_MS),
            Ordering::SeqCst,
        );
    }

    fn led_off_after_tap(&self) {
        caps::set_led(false);
        self.swallow_caps_led();
    }

    fn finish_cn_en_tap(&self) {
        CAPS_HELD.store(false, Ordering::SeqCst);
        if UPPER.load(Ordering::SeqCst) {
            return;
        }
        PRESS_GEN.fetch_add(1, Ordering::SeqCst);
        self.led_off_after_tap();
    }

    fn begin_cn_en(&self) {
        let now = caps::now_ms();
        if now < IGNORE_CAPS_UNTIL.load(Ordering::SeqCst) {
            return;
        }
        if CAPS_HELD.load(Ordering::SeqCst)
            && now.saturating_sub(LAST_CAPS_AT.load(Ordering::SeqCst)) < SAME_PRESS_MS
        {
            return;
        }
        CAPS_HELD.store(true, Ordering::SeqCst);
        LAST_CAPS_AT.store(now, Ordering::SeqCst);
        self.toggle_language();
        self.led_off_after_tap();
        let gen = PRESS_GEN.fetch_add(1, Ordering::SeqCst) + 1;
        let ctrl = Retained::into_raw(self.retain()) as usize;
        std::thread::spawn(move || {
            let start = caps::now_ms();
            // After set_led(false), a still-down reading is the physical key (not lock).
            let mut saw_physical = false;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(POLL_MS));
                if PRESS_GEN.load(Ordering::SeqCst) != gen {
                    unsafe {
                        let _ = Retained::<BuluoInputController>::from_raw(ctrl as *mut _);
                    }
                    return;
                }
                let elapsed = caps::now_ms().saturating_sub(start);
                let down = caps::lang_key_down();
                if down {
                    saw_physical = true;
                } else if saw_physical {
                    CAPS_HELD.store(false, Ordering::SeqCst);
                    if !UPPER.load(Ordering::SeqCst) {
                        PRESS_GEN.fetch_add(1, Ordering::SeqCst);
                        caps::set_led(false);
                        IGNORE_CAPS_UNTIL.store(
                            caps::now_ms().saturating_add(LED_SWALLOW_MS),
                            Ordering::SeqCst,
                        );
                    }
                    unsafe {
                        let _ = Retained::<BuluoInputController>::from_raw(ctrl as *mut _);
                    }
                    return;
                }
                if elapsed >= LONG_PRESS_MS {
                    let job = Box::new(LongPressJob { ctrl, gen });
                    unsafe {
                        dispatch_async_f(
                            std::ptr::addr_of_mut!(_dispatch_main_q) as *mut c_void,
                            Box::into_raw(job) as *mut c_void,
                            apply_long_press,
                        );
                    }
                    return;
                }
            }
        });
    }

    fn handle_shift(&self, event: &NSEvent) -> bool {
        let shift = event
            .modifierFlags()
            .contains(NSEventModifierFlags::Shift);
        let now = caps::now_ms();
        if shift && self.ivars().shift_down_at.get() == 0 {
            self.ivars().shift_down_at.set(now);
            self.ivars().shift_used.set(false);
            return false;
        }
        if !shift && self.ivars().shift_down_at.get() != 0 {
            let down = self.ivars().shift_down_at.get();
            self.ivars().shift_down_at.set(0);
            let held = now.saturating_sub(down);
            if !self.ivars().shift_used.get() && held <= SHIFT_TAP_MS {
                PRESS_GEN.fetch_add(1, Ordering::SeqCst);
                CAPS_HELD.store(false, Ordering::SeqCst);
                self.toggle_language();
                self.led_off_after_tap();
                return true;
            }
        }
        false
    }

    fn handle_flags(&self, event: &NSEvent) -> bool {
        let key = event.keyCode();
        if key == LEFT_SHIFT
            || key == RIGHT_SHIFT
            || self.ivars().shift_down_at.get() != 0
            || event
                .modifierFlags()
                .contains(NSEventModifierFlags::Shift)
        {
            let consumed = self.handle_shift(event);
            if key == LEFT_SHIFT || key == RIGHT_SHIFT {
                return consumed;
            }
        }
        let caps = event
            .modifierFlags()
            .contains(NSEventModifierFlags::CapsLock);
        let caps_changed = caps != self.ivars().last_caps.get();
        self.ivars().last_caps.set(caps);
        if caps_changed && caps {
            self.begin_cn_en();
            return true;
        }
        false
    }

    fn handle_key(&self, event: &NSEvent, client: &AnyObject) -> bool {
        let flags = event.modifierFlags();
        if flags.intersects(NSEventModifierFlags::Command | NSEventModifierFlags::Control) {
            return false;
        }
        PRESS_GEN.fetch_add(1, Ordering::SeqCst);
        if CAPS_HELD.swap(false, Ordering::SeqCst) && !UPPER.load(Ordering::SeqCst) {
            self.led_off_after_tap();
        }
        // KeyDown can arrive before FlagsChanged(Shift down). Mark the press
        // used *and* record shift_down_at so a late FlagsChanged does not
        // reset shift_used and treat Shift+letter as a 中/英 tap.
        let shift = event
            .modifierFlags()
            .contains(NSEventModifierFlags::Shift);
        if shift {
            self.ivars().shift_used.set(true);
            if self.ivars().shift_down_at.get() == 0 {
                self.ivars().shift_down_at.set(caps::now_ms());
            }
        } else if self.ivars().shift_down_at.get() != 0 {
            self.ivars().shift_used.set(true);
        }
        self.sync_mode();

        let ascii = self.ivars().engine.borrow().ascii_mode();
        let caps_upper = self.ivars().caps_upper.get();
        let Some(key) = map_key(
            event,
            !self.ivars().engine.borrow().candidates().is_empty(),
            ascii,
            caps_upper,
        ) else {
            return false;
        };

        let out = {
            let mut engine = self.ivars().engine.borrow_mut();
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| engine.handle_key(key)))
            {
                Ok(out) => out,
                Err(_) => {
                    eprintln!("buluo: engine panicked; keeping composing");
                    SessionOutput {
                        consumed: true,
                        commit: None,
                        marked: engine.composing().to_string(),
                        candidates: Vec::new(),
                        page: 0,
                        page_count: 0,
                        ascii_mode: engine.ascii_mode(),
                    }
                }
            }
        };
        if !out.consumed {
            return false;
        }
        apply_output(self, client, &out);
        true
    }
}

fn apply_output(ctrl: &BuluoInputController, client: &AnyObject, out: &SessionOutput) {
    // Engine left English (Space/Enter after a word like RUT) → Caps LED off.
    if ASCII.load(Ordering::SeqCst) && !out.ascii_mode {
        ctrl.set_ascii(false, false);
        caps::set_led(false);
        ctrl.swallow_caps_led();
    }

    let replacement = NSRange {
        location: NSNotFound as usize,
        length: NSNotFound as usize,
    };

    // The blinking insertion caret, before we rewrite marked text.
    let caret_before = blinking_caret(client);

    if let Some(text) = &out.commit {
        let s = NSString::from_str(text);
        unsafe {
            let _: () = msg_send![client, insertText: &*s, replacementRange: replacement];
        }
    }

    if out.marked.is_empty() {
        let empty = NSString::from_str("");
        unsafe {
            let _: () = msg_send![
                client,
                setMarkedText: &*empty,
                selectionRange: NSRange { location: 0, length: 0 },
                replacementRange: replacement
            ];
        }
    } else {
        let s = NSString::from_str(&out.marked);
        let caret = out.marked.encode_utf16().count();
        unsafe {
            let _: () = msg_send![
                client,
                setMarkedText: &*s,
                selectionRange: NSRange { location: caret, length: 0 },
                replacementRange: replacement
            ];
        }
    }

    // After setMarkedText, character indexes are relative to the marked string.
    let caret_after = if out.marked.is_empty() {
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0))
    } else {
        first_rect(
            client,
            NSRange {
                location: 0,
                length: 0,
            },
        )
    };
    let caret = pick_caret(caret_before, caret_after);

    let mtm = objc2::MainThreadMarker::new().expect("IME on main thread");
    let mut panel_slot = ctrl.ivars().panel.borrow_mut();
    if panel_slot.is_none() {
        *panel_slot = Some(CandidatePanel::new(mtm));
    }
    if let Some(panel) = panel_slot.as_ref() {
        if out.candidates.is_empty() {
            panel.hide();
        } else {
            panel.show(&out.candidates, caret);
        }
    }
}

fn blinking_caret(client: &AnyObject) -> NSRect {
    let selected: NSRange = unsafe { msg_send![client, selectedRange] };
    if range_ok(selected) {
        if let Some(rect) = usable_caret(to_screen(client, first_rect(client, selected))) {
            return rect;
        }
    }
    if let Some(rect) = line_height_rect(client) {
        return rect;
    }
    first_rect(client, NSRange { location: 0, length: 0 })
}

fn pick_caret(before: NSRect, after: NSRect) -> NSRect {
    let before_ok = usable_caret(before).is_some();
    let after_ok = usable_caret(after).is_some();
    if before_ok && after_ok {
        let dx = before.origin.x - after.origin.x;
        let dy = before.origin.y - after.origin.y;
        // Relative marked-text indexes can land far away; keep the blinking caret.
        if (dx * dx + dy * dy).sqrt() > 240.0 {
            return before;
        }
        return after;
    }
    if before_ok {
        return before;
    }
    after
}

fn range_ok(range: NSRange) -> bool {
    range.location != NSNotFound as usize
}

fn first_rect(client: &AnyObject, range: NSRange) -> NSRect {
    let mut actual = NSRange {
        location: 0,
        length: 0,
    };
    unsafe {
        msg_send![
            client,
            firstRectForCharacterRange: range,
            actualRange: &mut actual
        ]
    }
}

fn to_screen(client: &AnyObject, rect: NSRect) -> NSRect {
    let sel = objc2::sel!(window);
    let responds: bool = unsafe { msg_send![client, respondsToSelector: sel] };
    if !responds {
        return rect;
    }
    let window: Option<Retained<AnyObject>> = unsafe { msg_send![client, window] };
    let Some(window) = window else {
        return rect;
    };
    let converted: NSRect = unsafe { msg_send![&*window, convertRectToScreen: rect] };
    if usable_caret(converted).is_some() {
        converted
    } else {
        rect
    }
}

fn line_height_rect(client: &AnyObject) -> Option<NSRect> {
    let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
    let _: *mut AnyObject = unsafe {
        msg_send![
            client,
            attributesForCharacterIndex: 0usize,
            lineHeightRectangle: &mut rect
        ]
    };
    usable_caret(to_screen(client, rect))
}

fn usable_caret(rect: NSRect) -> Option<NSRect> {
    if !rect.origin.x.is_finite() || !rect.origin.y.is_finite() {
        return None;
    }
    if rect.origin.x.abs() > 20_000.0 || rect.origin.y.abs() > 20_000.0 {
        return None;
    }
    let empty = rect.size.width.abs() < 0.5
        && rect.size.height.abs() < 0.5
        && rect.origin.x.abs() < 1.0
        && rect.origin.y.abs() < 1.0;
    if empty {
        return None;
    }
    let mut rect = rect;
    if rect.size.height.abs() < 1.0 {
        rect.size.height = 16.0;
    }
    Some(rect)
}

/// Map a letter key to the character the engine should see.
///
/// Chinese: uppercase only when Shift is actually held *and* the event's
/// `characters()` is a capital (or missing). A stuck Shift *flag* with a
/// lowercase `characters()` stays pinyin — that is what made 打不出来 after
/// the last install if every letter was sent as `Char('Z')`.
fn mapped_letter(
    raw: char,
    displayed: Option<char>,
    shift: bool,
    ascii: bool,
    caps_upper: bool,
) -> char {
    if ascii {
        return if caps_upper || shift {
            raw.to_ascii_uppercase()
        } else {
            raw.to_ascii_lowercase()
        };
    }
    match displayed {
        Some(d) if d.is_ascii_uppercase() && shift => raw.to_ascii_uppercase(),
        Some(d) if d.is_ascii_alphabetic() => d.to_ascii_lowercase(),
        _ if shift => raw.to_ascii_uppercase(),
        _ => raw.to_ascii_lowercase(),
    }
}

fn map_char(ch: char, candidates_active: bool, ascii: bool) -> Option<KeyEvent> {
    match ch {
        c if c.is_ascii_alphabetic() => Some(KeyEvent::Char(c)),
        ' ' => Some(KeyEvent::Space),
        d @ '0'..='9' => Some(KeyEvent::Digit(d as u8 - b'0')),
        ',' | '，' if candidates_active => Some(KeyEvent::PagePrev),
        // `.` stays Punct so the engine can page *and* reinterpret as URL
        // when the next key is a letter (`taobao.` + `c` → `taobao.c`).
        '-' | '－' if candidates_active => Some(KeyEvent::PagePrev),
        '+' | '＋' | '=' if candidates_active => Some(KeyEvent::PageNext),
        '\'' | '\u{2018}' | '\u{2019}' if !ascii => Some(KeyEvent::Separator),
        c if engine::punct::is_punct_key(c) => Some(KeyEvent::Punct(c)),
        '\u{7f}' | '\u{8}' => Some(KeyEvent::Backspace),
        '\r' | '\n' => Some(KeyEvent::Enter),
        '\u{1b}' => Some(KeyEvent::Escape),
        _ => None,
    }
}

fn map_key(
    event: &NSEvent,
    candidates_active: bool,
    ascii: bool,
    caps_upper: bool,
) -> Option<KeyEvent> {
    let displayed = event.characters().map(|s| s.to_string());
    let displayed_ch = displayed.as_deref().and_then(|s| s.chars().next());
    let raw = event
        .charactersIgnoringModifiers()
        .and_then(|s| s.to_string().chars().next());
    let shift = event
        .modifierFlags()
        .contains(NSEventModifierFlags::Shift);

    if let Some(c) = raw.filter(|c| c.is_ascii_alphabetic()) {
        let letter = mapped_letter(c, displayed_ch, shift, ascii, caps_upper);
        return Some(KeyEvent::Char(letter));
    }

    if let Some(c) = displayed_ch {
        if c.is_ascii_alphabetic() {
            return Some(KeyEvent::Char(mapped_letter(
                c,
                displayed_ch,
                shift,
                ascii,
                caps_upper,
            )));
        }
        if let Some(key) = map_char(c, candidates_active, ascii) {
            return Some(key);
        }
    }
    match (raw, event.keyCode()) {
        (Some('-') | Some('－'), _) if candidates_active => Some(KeyEvent::PagePrev),
        (Some('+') | Some('＋') | Some('='), _) if candidates_active => Some(KeyEvent::PageNext),
        (_, 51) => Some(KeyEvent::Backspace),
        (_, 36 | 76) => Some(KeyEvent::Enter),
        (_, 53) => Some(KeyEvent::Escape),
        (_, CAPS_LOCK_KEY | EISU_KEY) => None,
        _ => None,
    }
}

fn open_engine() -> Engine {
    let system = bundle_path("system", "dict");
    let support = support_dir();
    match Engine::open(system.as_deref(), &support) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("buluo: failed to open engine ({e}), using builtin lexicon");
            let tsv = engine::dictionary::system::builtin_tsv();
            let entries = engine::dictionary::parse_tsv(tsv).unwrap_or_default();
            Engine::in_memory(entries)
        }
    }
}

fn bundle_path(name: &str, ext: &str) -> Option<PathBuf> {
    let bundle = NSBundle::mainBundle();
    let n = NSString::from_str(name);
    let e = NSString::from_str(ext);
    bundle
        .pathForResource_ofType(Some(&n), Some(&e))
        .map(|p| PathBuf::from(p.to_string()))
}

fn support_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Application Support/部落输入法")
}

pub fn register_controller_class() {
    let _ = BuluoInputController::class();
}

#[cfg(test)]
mod tests {
    use super::mapped_letter;

    #[test]
    fn chinese_unshifted_is_always_lowercase() {
        for raw in ['z', 'Z', 'n', 'N'] {
            assert_eq!(
                mapped_letter(raw, Some(raw.to_ascii_lowercase()), false, false, false),
                raw.to_ascii_lowercase()
            );
        }
    }

    #[test]
    fn chinese_stuck_shift_flag_with_lowercase_display_is_pinyin() {
        assert_eq!(
            mapped_letter('z', Some('z'), true, false, false),
            'z'
        );
        assert_eq!(
            mapped_letter('Z', Some('z'), true, false, false),
            'z'
        );
    }

    #[test]
    fn chinese_shift_and_uppercase_display_is_ascii() {
        assert_eq!(mapped_letter('a', Some('A'), true, false, false), 'A');
        assert_eq!(mapped_letter('A', Some('A'), true, false, false), 'A');
    }
}
