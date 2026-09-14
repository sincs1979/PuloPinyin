//! Caps Lock lamp: *set* lock state (IOHID). Never post another Caps Lock key.
//!
//! Do not raise `CapsLockDelayOverride` — HID then withholds Caps Lock until the
//! delay elapses, so a tap never reaches IMK. Language switch is a software tap
//! on the first Caps event; the 3s lamp/uppercase path is also software.
//! Do not poll `set_led(false)` while the key is held (that kills a long-press lamp).

use std::ffi::{c_char, c_void};

const CAPS_LOCK_STATE: i32 = 1;
const HID_PARAM_CONNECT: u32 = 1;
const CF_NUM_SINT32: i32 = 3;
const HID_KEYBOARD_PAGE: u32 = 0x01;
const HID_KEYBOARD_USAGE: u32 = 0x06;

#[link(name = "IOKit", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kIOMainPortDefault: u32;
    static mach_task_self_: u32;
    static kCFBooleanTrue: *const c_void;
    static kCFBooleanFalse: *const c_void;

    fn IOServiceGetMatchingService(main_port: u32, matching: *mut c_void) -> u32;
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceOpen(service: u32, owning_task: u32, connect_type: u32, connect: *mut u32) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOServiceClose(connect: u32) -> i32;
    fn IOHIDSetModifierLockState(connect: u32, selector: i32, state: u8) -> i32;

    fn IOHIDEventSystemClientCreateSimpleClient(allocator: *const c_void) -> *mut c_void;
    fn IOHIDEventSystemClientSetProperty(
        client: *mut c_void,
        key: *const c_void,
        property: *const c_void,
    ) -> u8;
    fn IOHIDEventSystemClientCopyServices(client: *mut c_void) -> *mut c_void;
    fn IOHIDServiceClientSetProperty(
        service: *mut c_void,
        key: *const c_void,
        property: *const c_void,
    ) -> u8;
    fn IOHIDServiceClientConformsTo(service: *mut c_void, usage_page: u32, usage: u32) -> u8;

    fn CFRelease(cf: *const c_void);
    fn CFNumberCreate(allocator: *const c_void, the_type: i32, value: *const i32) -> *mut c_void;
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, idx: isize) -> *mut c_void;
    fn __CFStringMakeConstantString(c: *const c_char) -> *const c_void;
}

fn cfstr(s: &[u8]) -> *const c_void {
    unsafe { __CFStringMakeConstantString(s.as_ptr() as *const c_char) }
}

fn hid_connect() -> Option<u32> {
    unsafe {
        let matching = IOServiceMatching(b"IOHIDSystem\0".as_ptr() as *const c_char);
        if matching.is_null() {
            return None;
        }
        let service = IOServiceGetMatchingService(kIOMainPortDefault, matching);
        if service == 0 {
            return None;
        }
        let mut connect = 0u32;
        let kr = IOServiceOpen(service, mach_task_self_, HID_PARAM_CONNECT, &mut connect);
        IOObjectRelease(service);
        if kr != 0 || connect == 0 {
            None
        } else {
            Some(connect)
        }
    }
}

fn set_service_caps(on: bool) {
    unsafe {
        let client = IOHIDEventSystemClientCreateSimpleClient(std::ptr::null());
        if client.is_null() {
            return;
        }
        let key = cfstr(b"HIDCapsLockState\0");
        let value = if on { kCFBooleanTrue } else { kCFBooleanFalse };
        IOHIDEventSystemClientSetProperty(client, key, value);
        let services = IOHIDEventSystemClientCopyServices(client);
        if !services.is_null() {
            let n = CFArrayGetCount(services);
            for i in 0..n {
                let svc = CFArrayGetValueAtIndex(services, i);
                if svc.is_null() {
                    continue;
                }
                if IOHIDServiceClientConformsTo(svc, HID_KEYBOARD_PAGE, HID_KEYBOARD_USAGE) != 0 {
                    IOHIDServiceClientSetProperty(svc, key, value);
                }
            }
            CFRelease(services);
        }
        CFRelease(client);
    }
}

pub fn set_led(on: bool) {
    if let Some(connect) = hid_connect() {
        unsafe {
            IOHIDSetModifierLockState(connect, CAPS_LOCK_STATE, u8::from(on));
            IOServiceClose(connect);
        }
    }
    set_service_caps(on);
}

fn apply_delay(ms: i32) {
    unsafe {
        let client = IOHIDEventSystemClientCreateSimpleClient(std::ptr::null());
        if client.is_null() {
            return;
        }
        let key = cfstr(b"CapsLockDelayOverride\0");
        let num = CFNumberCreate(std::ptr::null(), CF_NUM_SINT32, &ms);
        if !num.is_null() {
            IOHIDEventSystemClientSetProperty(client, key, num);
            let services = IOHIDEventSystemClientCopyServices(client);
            if !services.is_null() {
                let n = CFArrayGetCount(services);
                for i in 0..n {
                    let svc = CFArrayGetValueAtIndex(services, i);
                    if svc.is_null() {
                        continue;
                    }
                    if IOHIDServiceClientConformsTo(svc, HID_KEYBOARD_PAGE, HID_KEYBOARD_USAGE) != 0
                    {
                        IOHIDServiceClientSetProperty(svc, key, num);
                    }
                }
                CFRelease(services);
            }
            CFRelease(num);
        }
        CFRelease(client);
    }
}

/// Clear leftover HID Caps Lock delay so a tap delivers FlagsChanged immediately.
/// Never write a multi-second `CapsLockDelayOverride` — that blocks tap events.
pub fn ensure_immediate_caps() {
    apply_delay(0);
}

const HID_EVENT_STATE: u32 = 1;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state_id: u32, key: u16) -> bool;
}

/// Physical Caps Lock / 英数 key down (HID). Used to end a tap without KeyUp.
pub fn lang_key_down() -> bool {
    unsafe { CGEventSourceKeyState(HID_EVENT_STATE, 57) || CGEventSourceKeyState(HID_EVENT_STATE, 102) }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
