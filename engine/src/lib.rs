//! 部落输入法 core engine.
//!
//! Keyboard-independent: pinyin parsing, dictionary lookup, ranking, and learning.

pub mod candidate;
pub mod dictionary;
pub mod engine;
pub mod learning;
pub mod pinyin;
pub mod punct;
pub mod ranking;
pub mod symbols;

pub use candidate::{Candidate, CandidateList, PAGE_SIZE};
pub use dictionary::{BinaryDict, DictEntry};
pub use engine::{Engine, KeyEvent, SessionOutput};
pub use pinyin::{
    greedy_syllable_cover, match_consumed, matches_prefix, matches_word, Segment, Syllable,
};

/// User data directory for `learned.db` / `user.dict`.
pub fn default_support_dir() -> std::path::PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        std::path::PathBuf::from(home).join("Library/Application Support/部落输入法")
    } else if cfg!(windows) {
        let base = std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("APPDATA"))
            .or_else(|_| {
                std::env::var("USERPROFILE").map(|p| format!("{p}\\AppData\\Local"))
            })
            .unwrap_or_else(|_| r"C:\ProgramData".into());
        std::path::PathBuf::from(base).join("BuluoIME")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let base = std::env::var("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from(&home).join(".local/share"));
        base.join("buluo-ime")
    }
}

/// Best-effort path to a compiled `system.dict` (repo, install prefix, or none).
pub fn default_system_dict() -> Option<std::path::PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/system.dict"),
    );
    candidates.push(default_support_dir().join("system.dict"));
    candidates.push(std::path::PathBuf::from("/usr/share/buluo-ime/system.dict"));
    candidates.push(std::path::PathBuf::from("/usr/local/share/buluo-ime/system.dict"));
    if let Ok(pf) = std::env::var("ProgramFiles") {
        candidates.push(std::path::PathBuf::from(pf).join("BuluoIME/system.dict"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(std::path::PathBuf::from(local).join("BuluoIME/system.dict"));
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = std::path::PathBuf::from(home);
        candidates.push(
            home.join("Library/Input Methods/BuluoIME.app/Contents/Resources/system.dict"),
        );
        candidates.push(home.join(".local/share/fcitx5/buluo/system.dict"));
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let profile = std::path::PathBuf::from(profile);
        candidates.push(profile.join("AppData/Local/BuluoIME/system.dict"));
    }
    candidates.into_iter().find(|p| p.exists())
}

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Debug)]
pub enum EngineError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Dict(String),
    Format(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Io(e) => write!(f, "io: {e}"),
            EngineError::Sqlite(e) => write!(f, "sqlite: {e}"),
            EngineError::Dict(e) => write!(f, "dict: {e}"),
            EngineError::Format(e) => write!(f, "format: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        EngineError::Io(e)
    }
}

impl From<rusqlite::Error> for EngineError {
    fn from(e: rusqlite::Error) -> Self {
        EngineError::Sqlite(e)
    }
}

#[cfg(test)]
mod platform_paths {
    #[test]
    fn default_support_dir_points_at_product_folder() {
        let s = super::default_support_dir().to_string_lossy().into_owned();
        assert!(!s.is_empty());
        #[cfg(windows)]
        assert!(s.contains("BuluoIME"), "{s}");
        #[cfg(target_os = "macos")]
        assert!(s.contains("部落") || s.contains("Application Support"), "{s}");
        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(s.contains("buluo-ime"), "{s}");
    }
}
