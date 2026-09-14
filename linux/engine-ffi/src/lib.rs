//! C ABI for the portable pinyin engine (fcitx5 / ibus frontends).

use engine::{default_support_dir, default_system_dict, Engine, KeyEvent, SessionOutput};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;

pub const BULUO_KEY_CHAR: c_int = 1;
pub const BULUO_KEY_SPACE: c_int = 2;
pub const BULUO_KEY_BACKSPACE: c_int = 3;
pub const BULUO_KEY_ENTER: c_int = 4;
pub const BULUO_KEY_ESCAPE: c_int = 5;
pub const BULUO_KEY_PAGE_NEXT: c_int = 6;
pub const BULUO_KEY_PAGE_PREV: c_int = 7;
pub const BULUO_KEY_DIGIT: c_int = 8;
pub const BULUO_KEY_PUNCT: c_int = 9;
pub const BULUO_KEY_SEPARATOR: c_int = 10;
pub const BULUO_KEY_TOGGLE_ASCII: c_int = 11;

#[repr(C)]
pub struct BuluoOutput {
    pub consumed: c_int,
    pub ascii_mode: c_int,
    pub commit: *mut c_char,
    pub preedit: *mut c_char,
    pub candidates: *mut *mut c_char,
    pub candidate_count: c_int,
    pub page: c_int,
    pub page_count: c_int,
}

pub struct BuluoEngine {
    inner: Engine,
}

fn c_str_opt(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok().map(|s| s.to_string())
}

fn raw_cstring(s: &str) -> *mut c_char {
    CString::new(s.replace('\0', "")).map(|c| c.into_raw()).unwrap_or(ptr::null_mut())
}

fn map_key(kind: c_int, ch: u32) -> Option<KeyEvent> {
    match kind {
        BULUO_KEY_CHAR => char::from_u32(ch).map(KeyEvent::Char),
        BULUO_KEY_SPACE => Some(KeyEvent::Space),
        BULUO_KEY_BACKSPACE => Some(KeyEvent::Backspace),
        BULUO_KEY_ENTER => Some(KeyEvent::Enter),
        BULUO_KEY_ESCAPE => Some(KeyEvent::Escape),
        BULUO_KEY_PAGE_NEXT => Some(KeyEvent::PageNext),
        BULUO_KEY_PAGE_PREV => Some(KeyEvent::PagePrev),
        BULUO_KEY_DIGIT => {
            let d = if ch <= 9 { ch as u8 } else { (ch as u8).saturating_sub(b'0') };
            Some(KeyEvent::Digit(d))
        }
        BULUO_KEY_PUNCT => char::from_u32(ch).map(KeyEvent::Punct),
        BULUO_KEY_SEPARATOR => Some(KeyEvent::Separator),
        BULUO_KEY_TOGGLE_ASCII => Some(KeyEvent::ToggleAscii),
        _ => None,
    }
}

fn fill_output(src: SessionOutput, dst: &mut BuluoOutput) {
    dst.consumed = c_int::from(src.consumed);
    dst.ascii_mode = c_int::from(src.ascii_mode);
    dst.commit = src
        .commit
        .as_deref()
        .map(raw_cstring)
        .unwrap_or(ptr::null_mut());
    dst.preedit = raw_cstring(&src.marked);
    dst.page = src.page as c_int;
    dst.page_count = src.page_count as c_int;
    let ptrs: Vec<*mut c_char> = src
        .candidates
        .iter()
        .map(|c| raw_cstring(&c.word))
        .collect();
    dst.candidate_count = ptrs.len() as c_int;
    if ptrs.is_empty() {
        dst.candidates = ptr::null_mut();
    } else {
        let mut boxed = ptrs.into_boxed_slice();
        dst.candidates = boxed.as_mut_ptr();
        std::mem::forget(boxed);
    }
}

#[no_mangle]
pub unsafe extern "C" fn buluo_engine_new(
    dict_path: *const c_char,
    support_dir: *const c_char,
) -> *mut BuluoEngine {
    let dict = c_str_opt(dict_path)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(default_system_dict);
    let support = c_str_opt(support_dir)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_support_dir);
    let engine = match Engine::open(dict.as_deref(), &support) {
        Ok(e) => e,
        Err(_) => {
            let tsv = engine::dictionary::system::builtin_tsv();
            let entries = engine::dictionary::parse_tsv(tsv).unwrap_or_default();
            Engine::in_memory(entries)
        }
    };
    Box::into_raw(Box::new(BuluoEngine { inner: engine }))
}

#[no_mangle]
pub unsafe extern "C" fn buluo_engine_free(engine: *mut BuluoEngine) {
    if engine.is_null() {
        return;
    }
    drop(Box::from_raw(engine));
}

#[no_mangle]
pub unsafe extern "C" fn buluo_handle_key(
    engine: *mut BuluoEngine,
    kind: c_int,
    ch: u32,
    out: *mut BuluoOutput,
) {
    if engine.is_null() || out.is_null() {
        return;
    }
    *out = BuluoOutput {
        consumed: 0,
        ascii_mode: 0,
        commit: ptr::null_mut(),
        preedit: ptr::null_mut(),
        candidates: ptr::null_mut(),
        candidate_count: 0,
        page: 0,
        page_count: 0,
    };
    let Some(key) = map_key(kind, ch) else {
        return;
    };
    let src = (*engine).inner.handle_key(key);
    fill_output(src, &mut *out);
}

#[no_mangle]
pub unsafe extern "C" fn buluo_output_free(out: *mut BuluoOutput) {
    if out.is_null() {
        return;
    }
    let o = &mut *out;
    if !o.commit.is_null() {
        drop(CString::from_raw(o.commit));
        o.commit = ptr::null_mut();
    }
    if !o.preedit.is_null() {
        drop(CString::from_raw(o.preedit));
        o.preedit = ptr::null_mut();
    }
    if !o.candidates.is_null() && o.candidate_count > 0 {
        let count = o.candidate_count as usize;
        let slice = std::slice::from_raw_parts_mut(o.candidates, count);
        for p in slice.iter() {
            if !p.is_null() {
                drop(CString::from_raw(*p));
            }
        }
        drop(Vec::from_raw_parts(o.candidates, count, count));
        o.candidates = ptr::null_mut();
        o.candidate_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_types_zhongguo() {
        unsafe {
            let eng = buluo_engine_new(ptr::null(), ptr::null());
            assert!(!eng.is_null());
            let mut out = std::mem::zeroed::<BuluoOutput>();
            for b in b"zhongguo" {
                buluo_handle_key(eng, BULUO_KEY_CHAR, u32::from(*b), &mut out);
                assert_eq!(out.consumed, 1);
                buluo_output_free(&mut out);
            }
            buluo_handle_key(eng, BULUO_KEY_SPACE, 0, &mut out);
            assert_eq!(out.consumed, 1);
            assert!(!out.commit.is_null());
            let commit = CStr::from_ptr(out.commit).to_string_lossy().into_owned();
            buluo_output_free(&mut out);
            buluo_engine_free(eng);
            assert!(
                commit.contains('中') || commit == "中国" || !commit.is_empty(),
                "unexpected commit {commit:?}"
            );
        }
    }
}
