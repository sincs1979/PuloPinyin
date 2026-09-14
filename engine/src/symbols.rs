//! Static table of common Chinese special symbols, keyed by pinyin.
//!
//! Lookup is an exact key match (full spelling or listed initials) after
//! stripping separators. Single letters never hit this table, so a first
//! keystroke such as `z` does not expand a symbol bucket.

use crate::candidate::Candidate;
use crate::pinyin::parser::{normalize, normalize_query};

/// One special-symbol mapping. Candidate text is the symbol itself.
#[derive(Debug, Clone, Copy)]
pub struct SymbolDef {
    pub symbol: &'static str,
    pub name: &'static str,
    pub syllables: &'static [&'static str],
    pub keys: &'static [&'static str],
}

/// Commonly used Chinese punctuation / symbols.
///
/// Table order is prepend order when several entries share a key (`dh`).
pub const SYMBOL_TABLE: &[SymbolDef] = &[
    SymbolDef {
        symbol: "\u{2026}",
        name: "省略号",
        syllables: &["sheng", "lue", "hao"],
        keys: &["slh", "shengluehao", "shenglvehao"],
    },
    SymbolDef {
        symbol: "、",
        name: "顿号",
        syllables: &["dun", "hao"],
        keys: &["dunhao", "dh"],
    },
    SymbolDef {
        symbol: "，",
        name: "逗号",
        syllables: &["dou", "hao"],
        keys: &["douhao", "dh"],
    },
    SymbolDef {
        symbol: "。",
        name: "句号",
        syllables: &["ju", "hao"],
        keys: &["juhao", "jh"],
    },
    SymbolDef {
        symbol: "；",
        name: "分号",
        syllables: &["fen", "hao"],
        keys: &["fenhao", "fh"],
    },
    SymbolDef {
        symbol: "？",
        name: "问号",
        syllables: &["wen", "hao"],
        keys: &["wenhao", "wh"],
    },
    SymbolDef {
        symbol: "！",
        name: "感叹号",
        syllables: &["gan", "tan", "hao"],
        keys: &["gantanhao", "gth", "tanhao"],
    },
    SymbolDef {
        symbol: "：",
        name: "冒号",
        syllables: &["mao", "hao"],
        keys: &["maohao", "mh"],
    },
    SymbolDef {
        symbol: "——",
        name: "破折号",
        syllables: &["po", "zhe", "hao"],
        keys: &["pozhehao", "pzh"],
    },
    SymbolDef {
        symbol: "《》",
        name: "书名号",
        syllables: &["shu", "ming", "hao"],
        keys: &["shuminghao", "smh"],
    },
    SymbolDef {
        symbol: "《",
        name: "左书名号",
        syllables: &["zuo", "shu", "ming", "hao"],
        keys: &["zuoshuminghao"],
    },
    SymbolDef {
        symbol: "》",
        name: "右书名号",
        syllables: &["you", "shu", "ming", "hao"],
        keys: &["youshuminghao"],
    },
    SymbolDef {
        symbol: "“”",
        name: "引号",
        syllables: &["yin", "hao"],
        keys: &["yinhao", "yh", "shuangyinhao", "syh"],
    },
    SymbolDef {
        symbol: "‘’",
        name: "单引号",
        syllables: &["dan", "yin", "hao"],
        keys: &["danyinhao", "dyh"],
    },
    SymbolDef {
        symbol: "（）",
        name: "括号",
        syllables: &["kuo", "hao"],
        keys: &["kuohao", "kh"],
    },
    SymbolDef {
        symbol: "（",
        name: "左括号",
        syllables: &["zuo", "kuo", "hao"],
        keys: &["zuokuohao", "zkh"],
    },
    SymbolDef {
        symbol: "）",
        name: "右括号",
        syllables: &["you", "kuo", "hao"],
        keys: &["youkuohao", "ykh"],
    },
    SymbolDef {
        symbol: "【】",
        name: "方括号",
        syllables: &["fang", "kuo", "hao"],
        keys: &["fakuohao", "fangkuohao", "fkh"],
    },
    SymbolDef {
        symbol: "【",
        name: "左方括号",
        syllables: &["zuo", "fang", "kuo", "hao"],
        keys: &["zuofakuohao", "zuofangkuohao"],
    },
    SymbolDef {
        symbol: "】",
        name: "右方括号",
        syllables: &["you", "fang", "kuo", "hao"],
        keys: &["youfakuohao", "youfangkuohao"],
    },
    SymbolDef {
        symbol: "·",
        name: "间隔号",
        syllables: &["jian", "ge", "hao"],
        keys: &["jiangehao", "jgh"],
    },
    SymbolDef {
        symbol: "－",
        name: "连接号",
        syllables: &["lian", "jie", "hao"],
        keys: &["lianjiehao", "ljh"],
    },
    SymbolDef {
        symbol: "％",
        name: "百分号",
        syllables: &["bai", "fen", "hao"],
        keys: &["baihao", "bfh", "baifenhao"],
    },
    SymbolDef {
        symbol: "°",
        name: "度数",
        syllables: &["du", "hao"],
        keys: &["duhao"],
    },
];

/// Triggers that open the full special-symbol catalog (特殊符号).
/// `tesufuhao` is accepted as a typo for `teshufuhao`.
pub const CATALOG_KEYS: &[&str] = &["tsfh", "tesufuhao", "teshufuhao"];

/// Fixed catalog order. Page size is [`crate::PAGE_SIZE`] (2×5).
/// Slot *i* is always `CATALOG[i]` — never re-ranked by frequency.
pub const CATALOG: &[&str] = &[
    "\u{2026}",
    "、",
    "，",
    "。",
    "；",
    "？",
    "！",
    "：",
    "——",
    "《",
    "》",
    "“",
    "”",
    "‘",
    "’",
    "（",
    "）",
    "【",
    "】",
    "·",
    "－",
    "￥",
    "％",
    "°",
    "※",
    "〜",
    "「",
    "」",
    "『",
    "』",
    "〈",
    "〉",
    "〔",
    "〕",
    "〖",
    "〗",
    "±",
    "×",
    "÷",
    "≠",
    "≤",
    "≥",
    "∞",
    "√",
];

const MIN_KEY_LEN: usize = 2;

fn compact_key(input: &str) -> String {
    normalize(input)
}

pub fn is_catalog_key(input: &str) -> bool {
    let key = compact_key(input);
    CATALOG_KEYS.iter().any(|k| *k == key)
}

/// Entries whose listed keys include the compact composing string.
pub fn lookup(input: &str) -> Vec<&'static SymbolDef> {
    let key = compact_key(input);
    if key.len() < MIN_KEY_LEN {
        return Vec::new();
    }
    SYMBOL_TABLE
        .iter()
        .filter(|def| def.keys.iter().any(|k| *k == key))
        .collect()
}

pub fn is_symbol_word(word: &str) -> bool {
    SYMBOL_TABLE.iter().any(|def| def.symbol == word)
        || CATALOG.iter().any(|sym| *sym == word)
}

/// Symbol candidates to prepend. `consumed` is in [`normalize_query`] space.
pub fn symbol_candidates(input: &str) -> Vec<Candidate> {
    let consumed = normalize_query(input).len();
    if consumed == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for def in lookup(input) {
        if out.iter().any(|c: &Candidate| c.word == def.symbol) {
            continue;
        }
        out.push(Candidate {
            word: def.symbol.to_string(),
            syllables: def.syllables.iter().map(|s| (*s).to_string()).collect(),
            base_frequency: 0,
            user_count: 0,
            last_used: None,
            score: 0.0,
            consumed,
            complete: true,
        });
    }
    out
}

/// Whole catalog in fixed table order. Does not consult frequency.
pub fn catalog_candidates(input: &str) -> Vec<Candidate> {
    let consumed = normalize_query(input).len();
    CATALOG
        .iter()
        .map(|sym| Candidate {
            word: (*sym).to_string(),
            syllables: vec![
                "te".into(),
                "shu".into(),
                "fu".into(),
                "hao".into(),
            ],
            base_frequency: 0,
            user_count: 0,
            last_used: None,
            score: 0.0,
            consumed,
            complete: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_at_least_two_letters() {
        for def in SYMBOL_TABLE {
            for key in def.keys {
                assert!(
                    key.len() >= MIN_KEY_LEN,
                    "{} key {key:?} would explode a first-letter bucket",
                    def.name
                );
                assert!(
                    key.chars().all(|c| c.is_ascii_lowercase()),
                    "{} key {key:?} must be lowercase ascii",
                    def.name
                );
            }
        }
    }

    #[test]
    fn single_letter_never_hits_table() {
        for ch in 'a'..='z' {
            assert!(
                lookup(&ch.to_string()).is_empty(),
                "{ch} must not look up symbols"
            );
        }
    }

    #[test]
    fn required_keys() {
        assert_eq!(lookup("slh")[0].symbol, "\u{2026}");
        assert_eq!(lookup("shengluehao")[0].symbol, "\u{2026}");
        assert_eq!(lookup("dunhao")[0].symbol, "、");
        assert_eq!(lookup("douhao")[0].symbol, "，");
        let dh: Vec<_> = lookup("dh").iter().map(|d| d.symbol).collect();
        assert_eq!(dh, ["、", "，"]);
    }

    #[test]
    fn ellipsis_is_horizontal_ellipsis() {
        assert_eq!(lookup("slh")[0].symbol.chars().next(), Some('\u{2026}'));
        assert_eq!(lookup("slh")[0].symbol.chars().count(), 1);
    }

    #[test]
    fn catalog_triggers() {
        assert!(is_catalog_key("tsfh"));
        assert!(is_catalog_key("tesufuhao"));
        assert!(is_catalog_key("teshufuhao"));
        assert!(is_catalog_key("te'shu'fu'hao"));
        assert!(!is_catalog_key("t"));
        assert!(!is_catalog_key("ts"));
        assert!(!is_catalog_key("tsf"));
        assert!(!is_catalog_key("slh"));
    }

    #[test]
    fn catalog_page1_is_first_ten() {
        assert_eq!(
            &CATALOG[..10],
            &[
                "\u{2026}", "、", "，", "。", "；", "？", "！", "：", "——", "《"
            ]
        );
        assert_eq!(
            &CATALOG[10..20],
            &["》", "“", "”", "‘", "’", "（", "）", "【", "】", "·"]
        );
        assert!(CATALOG.len() > 10, "catalog must spill onto later pages");
    }
}
