fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }
    let def = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ime_win.def");
    if def.exists() {
        println!("cargo:rerun-if-changed=ime_win.def");
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def.display());
    }
}
