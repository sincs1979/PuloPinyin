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

pub use candidate::{Candidate, CandidateList, PAGE_SIZE};
pub use dictionary::{BinaryDict, DictEntry};
pub use engine::{Engine, KeyEvent, SessionOutput};
pub use pinyin::{match_consumed, matches_prefix, matches_word, Segment, Syllable};

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
