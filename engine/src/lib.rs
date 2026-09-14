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
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    if cfg!(target_os = "macos") {
        std::path::PathBuf::from(home).join("Library/Application Support/部落输入法")
    } else {
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
    if let Ok(home) = std::env::var("HOME") {
        let home = std::path::PathBuf::from(home);
        candidates.push(
            home.join("Library/Input Methods/BuluoIME.app/Contents/Resources/system.dict"),
        );
        candidates.push(home.join(".local/share/fcitx5/buluo/system.dict"));
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
