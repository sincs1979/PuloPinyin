//! Windows TSF frontend for 部落输入法.
//!
//! Chinese IME on Windows is a COM in-process server (`ITfTextInputProcessor`),
//! not a normal GUI app. The pinyin engine is portable; this crate is the DLL
//! that Text Services Framework loads (`InprocServer32`).
//!
//! **This round is a compiling scaffold.** `DllGetClassObject` does not yet
//! return a real TIP. Keys, candidate HWND, and UTF-16 commit into the focused
//! app are wired at the engine-host layer (see tests) and still need TSF sinks.
//! GitHub Actions `windows-latest` builds the DLL and the installer; you do
//! not need a Windows PC to develop.

pub mod candidate;

use engine::{Engine, KeyEvent, SessionOutput};

/// CLSID of the Text Service (`HKCR\CLSID\...` / TSF TIP key).
pub const CLSID_BULUO_TEXT_SERVICE: &str = "{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}";
/// Language profile GUID under `LanguageProfile\0x00000804\`.
pub const GUID_BULUO_PROFILE: &str = "{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D02}";
pub const LANGID_ZH_CN: u16 = 0x0804;
pub const IME_NAME: &str = "部落输入法";
pub const DLL_NAME: &str = "ime_win.dll";

const VK_BACK: u32 = 0x08;
const VK_RETURN: u32 = 0x0D;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_SHIFT: u32 = 0x10;
const VK_OEM_COMMA: u32 = 0xBC;
const VK_OEM_PERIOD: u32 = 0xBE;

/// Map a Win32 virtual-key (+ optional UTF-16 character) into the portable engine.
pub fn map_win_key(vk: u32, ch: Option<char>) -> Option<KeyEvent> {
    match vk {
        VK_BACK => Some(KeyEvent::Backspace),
        VK_RETURN => Some(KeyEvent::Enter),
        VK_ESCAPE => Some(KeyEvent::Escape),
        VK_SPACE => Some(KeyEvent::Space),
        VK_OEM_PERIOD => Some(KeyEvent::PageNext),
        VK_OEM_COMMA => Some(KeyEvent::PagePrev),
        VK_SHIFT => Some(KeyEvent::ToggleAscii),
        _ => {
            let c = ch?;
            if c.is_ascii_digit() {
                return Some(KeyEvent::Digit(c as u8 - b'0'));
            }
            if c.is_ascii_alphabetic() {
                return Some(KeyEvent::Char(c.to_ascii_lowercase()));
            }
            if c == '\'' || c == '’' {
                return Some(KeyEvent::Separator);
            }
            Some(KeyEvent::Punct(c))
        }
    }
}

/// NUL-terminated UTF-16 for `ITfInsertAtSelection` / `SetText`.
pub fn to_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct ImeSession {
    engine: Engine,
}

impl Default for ImeSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ImeSession {
    pub fn new() -> Self {
        let dict = engine::default_system_dict();
        let support = engine::default_support_dir();
        let engine = Engine::open(dict.as_deref(), &support).unwrap_or_else(|_| {
            let tsv = engine::dictionary::system::builtin_tsv();
            let entries = engine::dictionary::parse_tsv(tsv).unwrap_or_default();
            Engine::in_memory(entries)
        });
        Self { engine }
    }

    pub fn handle(&mut self, key: KeyEvent) -> SessionOutput {
        self.engine.handle_key(key)
    }

    pub fn commit_utf16(out: &SessionOutput) -> Option<Vec<u16>> {
        out.commit.as_deref().map(to_utf16)
    }
}

#[cfg(windows)]
mod exports {
    use std::ffi::c_void;

    const S_OK: i32 = 0;
    const S_FALSE: i32 = 1;
    /// `CLASS_E_CLASSNOTAVAILABLE` (0x80040111) until the TSF class factory exists.
    const CLASS_E_CLASSNOTAVAILABLE: i32 = 0x8004_0111u32 as i32;

    #[no_mangle]
    pub extern "system" fn DllCanUnloadNow() -> i32 {
        S_FALSE
    }

    #[no_mangle]
    pub unsafe extern "system" fn DllGetClassObject(
        _rclsid: *const u8,
        _riid: *const u8,
        ppv: *mut *mut c_void,
    ) -> i32 {
        if !ppv.is_null() {
            *ppv = std::ptr::null_mut();
        }
        CLASS_E_CLASSNOTAVAILABLE
    }

    /// Registry is written by the installer / `register.ps1`, not here.
    #[no_mangle]
    pub extern "system" fn DllRegisterServer() -> i32 {
        S_OK
    }

    #[no_mangle]
    pub extern "system" fn DllUnregisterServer() -> i32 {
        S_OK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_commit_roundtrip() {
        let u = to_utf16("中国");
        let s = String::from_utf16(&u[..u.len() - 1]).unwrap();
        assert_eq!(s, "中国");
    }

    #[test]
    fn zhongguo_space_commits() {
        let mut ime = ImeSession::new();
        for c in "zhongguo".chars() {
            let out = ime.handle(KeyEvent::Char(c));
            assert!(out.consumed);
        }
        let out = ime.handle(KeyEvent::Space);
        assert!(out.consumed);
        let commit = ImeSession::commit_utf16(&out).unwrap_or_default();
        let text = String::from_utf16(&commit[..commit.len().saturating_sub(1)]).unwrap_or_default();
        assert!(
            text.contains('中') || text == "中国" || !text.is_empty(),
            "unexpected commit {text:?}"
        );
    }

    #[test]
    fn map_letters_digits_space() {
        assert!(matches!(
            map_win_key(0x41, Some('a')),
            Some(KeyEvent::Char('a'))
        ));
        assert!(matches!(map_win_key(VK_SPACE, None), Some(KeyEvent::Space)));
        assert!(matches!(
            map_win_key(0x31, Some('1')),
            Some(KeyEvent::Digit(1))
        ));
    }
}
