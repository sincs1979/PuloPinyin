#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

#[cfg(target_os = "macos")]
mod caps;
#[cfg(target_os = "macos")]
mod candidate_window;
#[cfg(target_os = "macos")]
mod controller;

#[cfg(target_os = "macos")]
fn main() {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::{NSBundle, NSString};
    use objc2_input_method_kit::IMKServer;

    let mtm = MainThreadMarker::new().expect("部落输入法 must start on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    controller::register_controller_class();

    let ident = NSBundle::mainBundle()
        .bundleIdentifier()
        .unwrap_or_else(|| NSString::from_str("com.buluo.inputmethod.pinyin"));
    let connection = NSString::from_str("com.buluo.inputmethod.pinyin_Connection");
    let server = unsafe {
        IMKServer::initWithName_bundleIdentifier(
            IMKServer::alloc(),
            Some(&connection),
            Some(&ident),
        )
    };
    let Some(_server) = server else {
        eprintln!("buluo: IMKServer failed to start");
        std::process::exit(1);
    };

    app.run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("部落输入法 is macOS-only.");
    std::process::exit(1);
}
