use crate::pinyin::parser::normalize;
use crate::EngineError;
use std::io::{Read, Write};
use std::path::Path;

pub const MAGIC: &[u8; 8] = b"PULODICT";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictEntry {
    pub word: String,
    pub syllables: Vec<String>,
    pub frequency: u32,
}

impl DictEntry {
    pub fn pinyin_spaced(&self) -> String {
        self.syllables.join(" ")
    }

    pub fn pinyin_compact(&self) -> String {
        self.syllables.concat()
    }
}

/// In-memory dictionary loaded from the compact binary format (or TSV).
#[derive(Debug, Clone, Default)]
pub struct BinaryDict {
    pub entries: Vec<DictEntry>,
    /// First-letter buckets for phrase (2+ syllables) lookup.
    pub by_first_letter: [Vec<u32>; 26],
    /// First-syllable buckets (exact, no fuzzy) for full-pinyin lookup.
    pub by_first_syllable: Vec<(String, Vec<u32>)>,
}

impl BinaryDict {
    pub fn from_entries(entries: Vec<DictEntry>) -> Self {
        let mut dict = BinaryDict {
            entries,
            by_first_letter: std::array::from_fn(|_| Vec::new()),
            by_first_syllable: Vec::new(),
        };
        dict.rebuild_index();
        dict
    }

    pub fn load(path: impl AsRef<Path>) -> crate::Result<Self> {
        let mut file = std::fs::File::open(path.as_ref())?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Self::from_bytes(&buf)
    }

    pub fn from_bytes(data: &[u8]) -> crate::Result<Self> {
        if data.len() < 16 || &data[0..8] != MAGIC {
            return Err(EngineError::Dict("invalid magic".into()));
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(EngineError::Dict(format!("unsupported version {version}")));
        }
        let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        let mut pos = 16;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            if pos + 8 > data.len() {
                return Err(EngineError::Dict("truncated entry header".into()));
            }
            let frequency = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            let word_len = u16::from_le_bytes(data[pos + 4..pos + 6].try_into().unwrap()) as usize;
            let pinyin_len =
                u16::from_le_bytes(data[pos + 6..pos + 8].try_into().unwrap()) as usize;
            pos += 8;
            if pos + word_len + pinyin_len > data.len() {
                return Err(EngineError::Dict("truncated entry body".into()));
            }
            let word = std::str::from_utf8(&data[pos..pos + word_len])
                .map_err(|_| EngineError::Dict("word is not utf-8".into()))?
                .to_string();
            pos += word_len;
            let pinyin = std::str::from_utf8(&data[pos..pos + pinyin_len])
                .map_err(|_| EngineError::Dict("pinyin is not utf-8".into()))?
                .to_string();
            pos += pinyin_len;
            let syllables = split_pinyin(&pinyin);
            entries.push(DictEntry {
                word,
                syllables,
                frequency,
            });
        }
        Ok(Self::from_entries(entries))
    }

    pub fn write_to(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let bytes = self.to_bytes();
        let mut file = std::fs::File::create(path.as_ref())?;
        file.write_all(&bytes)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for e in &self.entries {
            let pinyin = e.pinyin_spaced();
            buf.extend_from_slice(&e.frequency.to_le_bytes());
            buf.extend_from_slice(&(e.word.len() as u16).to_le_bytes());
            buf.extend_from_slice(&(pinyin.len() as u16).to_le_bytes());
            buf.extend_from_slice(e.word.as_bytes());
            buf.extend_from_slice(pinyin.as_bytes());
        }
        buf
    }

    fn rebuild_index(&mut self) {
        for bucket in &mut self.by_first_letter {
            bucket.clear();
        }
        use std::collections::HashMap;
        let mut first_syl: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, e) in self.entries.iter().enumerate() {
            let id = i as u32;
            if let Some(first) = e.syllables.first() {
                first_syl.entry(first.clone()).or_default().push(id);
                if e.syllables.len() >= 2 {
                    if let Some(c) = first.chars().next() {
                        if c.is_ascii_lowercase() {
                            self.by_first_letter[(c as u8 - b'a') as usize].push(id);
                        }
                    }
                    // also bucket fuzzy first letters so zhong is reachable via 'z' already,
                    // and zhang via 'z'. r/l handled at match time by scanning both buckets.
                }
            }
        }
        let mut pairs: Vec<(String, Vec<u32>)> = first_syl.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        self.by_first_syllable = pairs;
    }

    pub fn lookup_candidates(&self, input: &str) -> Vec<&DictEntry> {
        let letters = normalize(input);
        if letters.is_empty() {
            return Vec::new();
        }
        let mut ids = Vec::new();
        let mut seen = vec![false; self.entries.len()];

        let first = letters.chars().next().unwrap();
        if first.is_ascii_lowercase() {
            let idx = (first as u8 - b'a') as usize;
            for &id in &self.by_first_letter[idx] {
                push_id(&mut ids, &mut seen, id);
            }
            // fuzzy first-letter buckets
            for extra in fuzzy_first_letters(first) {
                let i = (extra as u8 - b'a') as usize;
                for &id in &self.by_first_letter[i] {
                    push_id(&mut ids, &mut seen, id);
                }
            }
        }

        // Full first-syllable: try longest prefixes of input as syllables.
        for (syl, bucket) in &self.by_first_syllable {
            if letters.starts_with(syl.as_str())
                || crate::pinyin::fuzzy::fuzzy_variants(syl)
                    .iter()
                    .any(|v| letters.starts_with(v))
            {
                for &id in bucket {
                    push_id(&mut ids, &mut seen, id);
                }
            }
        }

        self.finish_lookup(ids, input, false)
    }

    /// Words whose pinyin is a prefix of `input`, or whose pinyin `input` prefixes.
    ///
    /// Must not dump a whole first-letter bucket on a single keystroke. On a 390k
    /// lexicon that used to match ~35k `z*` words for `z` and freeze the IME.
    pub fn lookup_prefix(&self, input: &str) -> Vec<&DictEntry> {
        let letters = normalize(input);
        if letters.is_empty() {
            return Vec::new();
        }
        let mut consume_ids = Vec::new();
        let mut unfinished_ids = Vec::new();
        let mut seen = vec![false; self.entries.len()];
        let mut have_syllable_hit = false;

        for (syl, bucket) in &self.by_first_syllable {
            match syllable_lead_kind(&letters, syl) {
                SyllableLead::None => {}
                SyllableLead::Complete => {
                    have_syllable_hit = true;
                    let rest = strip_leading_syl(&letters, syl).unwrap_or("");
                    for &id in bucket {
                        let Some(e) = self.entries.get(id as usize) else {
                            continue;
                        };
                        if rest.is_empty() {
                            if e.syllables.len() <= 1 {
                                push_id(&mut consume_ids, &mut seen, id);
                            } else {
                                // `wo` → 我们 / 我的: unfinished longer words, cap later.
                                push_id(&mut unfinished_ids, &mut seen, id);
                            }
                        } else if complete_lead_worth_checking(rest, e) {
                            push_id(&mut consume_ids, &mut seen, id);
                        }
                    }
                }
                SyllableLead::Unfinished => {
                    have_syllable_hit = true;
                    for &id in bucket {
                        push_id(&mut unfinished_ids, &mut seen, id);
                    }
                }
            }
        }

        // Initials (`zg`, `wdmy`) only when no first syllable of the input is known.
        // Never scan the 30k-wide letter dump for a single letter.
        if !have_syllable_hit && letters.len() >= 2 {
            let first = letters.chars().next().unwrap();
            if first.is_ascii_lowercase() {
                let idx = (first as u8 - b'a') as usize;
                for &id in &self.by_first_letter[idx] {
                    if self
                        .entries
                        .get(id as usize)
                        .is_some_and(|e| letter_bucket_second_ok(&letters, e))
                    {
                        push_id(&mut consume_ids, &mut seen, id);
                    }
                }
                for extra in fuzzy_first_letters(first) {
                    let i = (extra as u8 - b'a') as usize;
                    for &id in &self.by_first_letter[i] {
                        if self
                            .entries
                            .get(id as usize)
                            .is_some_and(|e| letter_bucket_second_ok(&letters, e))
                        {
                            push_id(&mut consume_ids, &mut seen, id);
                        }
                    }
                }
            }
        }

        let mut out = self.finish_lookup(consume_ids, input, true);
        if !unfinished_ids.is_empty() {
            let mut incomplete: Vec<&DictEntry> = unfinished_ids
                .into_iter()
                .filter_map(|id| self.entries.get(id as usize))
                .collect();
            if incomplete.len() > MAX_PREFIX_INCOMPLETE {
                incomplete.sort_by(|a, b| {
                    b.frequency
                        .cmp(&a.frequency)
                        .then_with(|| a.word.cmp(&b.word))
                });
                incomplete.truncate(MAX_PREFIX_INCOMPLETE);
            }
            out.extend(incomplete);
        }
        out
    }

    fn finish_lookup<'a>(&'a self, ids: Vec<u32>, input: &str, prefix: bool) -> Vec<&'a DictEntry> {
        let mut complete = Vec::new();
        let mut incomplete = Vec::new();
        for id in ids {
            let Some(e) = self.entries.get(id as usize) else {
                continue;
            };
            if prefix {
                if crate::pinyin::match_consumed(input, &e.syllables).is_some() {
                    complete.push(e);
                } else if crate::pinyin::matches_prefix(input, &e.syllables) {
                    incomplete.push(e);
                }
            } else if crate::pinyin::matches_word(input, &e.syllables) {
                complete.push(e);
            }
        }
        if prefix && incomplete.len() > MAX_PREFIX_INCOMPLETE {
            incomplete.sort_by(|a, b| {
                b.frequency
                    .cmp(&a.frequency)
                    .then_with(|| a.word.cmp(&b.word))
            });
            incomplete.truncate(MAX_PREFIX_INCOMPLETE);
        }
        complete.extend(incomplete);
        complete
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

const MAX_PREFIX_INCOMPLETE: usize = 96;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SyllableLead {
    None,
    /// `wodemaya` starts with `wo` (or a fuzzy variant).
    Complete,
    /// `zhon` is an unfinished `zhong`. Never a single letter.
    Unfinished,
}

fn strip_leading_syl<'a>(input: &'a str, syl: &str) -> Option<&'a str> {
    if let Some(rest) = input.strip_prefix(syl) {
        return Some(rest);
    }
    for v in crate::pinyin::fuzzy::fuzzy_variants(syl) {
        if let Some(rest) = input.strip_prefix(v.as_str()) {
            return Some(rest);
        }
    }
    None
}

fn complete_lead_worth_checking(rest: &str, e: &DictEntry) -> bool {
    if rest.is_empty() || e.syllables.len() < 2 {
        return true;
    }
    let s1 = e.syllables[1].as_str();
    if s1.is_empty() {
        return false;
    }
    if rest.starts_with(s1) || s1.starts_with(rest) {
        return true;
    }
    let Some(c) = rest.chars().next() else {
        return true;
    };
    let Some(d) = s1.chars().next() else {
        return false;
    };
    d == c || fuzzy_second_letter(d, c)
}

fn syllable_lead_kind(input: &str, syl: &str) -> SyllableLead {
    if input.is_empty() || syl.is_empty() {
        return SyllableLead::None;
    }
    if input.starts_with(syl) {
        return SyllableLead::Complete;
    }
    let variants = crate::pinyin::fuzzy::fuzzy_variants(syl);
    if variants.iter().any(|v| input.starts_with(v.as_str())) {
        return SyllableLead::Complete;
    }
    if input.len() >= 2 && syl.starts_with(input) {
        return SyllableLead::Unfinished;
    }
    if input.len() >= 2 && variants.iter().any(|v| v.starts_with(input)) {
        return SyllableLead::Unfinished;
    }
    SyllableLead::None
}

fn letter_bucket_second_ok(input: &str, e: &DictEntry) -> bool {
    let Some(c2) = input.chars().nth(1) else {
        return false;
    };
    if let Some(syl) = e.syllables.first() {
        if let Some(s1) = syl.chars().nth(1) {
            if s1 == c2 {
                return true;
            }
            // `zh`/`ch`/`sh` as a two-letter initial on the first syllable.
            if c2 == 'h' && matches!(syl.chars().next(), Some('z' | 'c' | 's')) {
                return true;
            }
        }
    }
    if e.syllables.len() >= 2 {
        if let Some(c) = e.syllables[1].chars().next() {
            if c == c2 || fuzzy_second_letter(c, c2) {
                return true;
            }
        }
    }
    false
}

fn fuzzy_second_letter(a: char, b: char) -> bool {
    a == b || matches!((a, b), ('r', 'l') | ('l', 'r'))
}

fn push_id(ids: &mut Vec<u32>, seen: &mut [bool], id: u32) {
    let i = id as usize;
    if i < seen.len() && !seen[i] {
        seen[i] = true;
        ids.push(id);
    }
}

fn fuzzy_first_letters(c: char) -> Vec<char> {
    match c {
        'z' => vec!['z'], // zh already starts with z
        'c' => vec!['c'],
        's' => vec!['s'],
        'r' => vec!['l'],
        'l' => vec!['r'],
        _ => Vec::new(),
    }
}

/// Syllables used when a learned word is queried again.
///
/// ASCII words stay one token (`ChatGPT` → `["chatgpt"]`) so later pinyin
/// `chatgpt` matches via [`crate::pinyin::matches_word`]. Chinese phrases
/// still split on spaces / greedy syllables.
pub fn learned_syllables(word: &str, pinyin: &str) -> Vec<String> {
    if !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()) {
        vec![word.to_ascii_lowercase()]
    } else {
        split_pinyin(pinyin)
    }
}

pub fn split_pinyin(pinyin: &str) -> Vec<String> {
    let n = normalize(pinyin);
    if pinyin.contains(' ') || pinyin.contains('\'') {
        pinyin
            .split(|c: char| c == ' ' || c == '\'')
            .filter(|s| !s.is_empty())
            .map(|s| normalize(s))
            .filter(|s| !s.is_empty())
            .collect()
    } else if n.is_empty() {
        Vec::new()
    } else {
        // Split concatenated pinyin using greedy syllables.
        let mut rest = n.as_str();
        let mut out = Vec::new();
        while !rest.is_empty() {
            if let Some(s) = crate::pinyin::syllable::longest_syllable_at(rest) {
                out.push(s.to_string());
                rest = &rest[s.len()..];
            } else {
                out.push(rest.to_string());
                break;
            }
        }
        out
    }
}

pub fn parse_tsv(tsv: &str) -> crate::Result<Vec<DictEntry>> {
    let mut entries = Vec::new();
    for (lineno, line) in tsv.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 2 {
            return Err(EngineError::Format(format!(
                "line {}: expected word<TAB>pinyin[<TAB>freq]",
                lineno + 1
            )));
        }
        let word = cols[0].trim().to_string();
        let syllables = split_pinyin(cols[1]);
        if word.is_empty() || syllables.is_empty() {
            return Err(EngineError::Format(format!(
                "line {}: empty word/pinyin",
                lineno + 1
            )));
        }
        let frequency = if cols.len() >= 3 {
            cols[2].trim().parse::<u32>().unwrap_or(1)
        } else {
            1
        };
        entries.push(DictEntry {
            word,
            syllables,
            frequency,
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learned_ascii_word_stays_one_syllable() {
        assert_eq!(
            learned_syllables("ChatGPT", "chatgpt"),
            vec!["chatgpt".to_string()]
        );
        assert_eq!(
            learned_syllables("中国", "zhong guo"),
            vec!["zhong".to_string(), "guo".to_string()]
        );
    }

    #[test]
    fn roundtrip_bytes() {
        let entries = parse_tsv("中国\tzhong guo\t100\n是\tshi\t200\n").unwrap();
        let dict = BinaryDict::from_entries(entries);
        let bytes = dict.to_bytes();
        let loaded = BinaryDict::from_bytes(&bytes).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].word, "中国");
    }

    #[test]
    fn lookup_zg() {
        let entries = parse_tsv("中国\tzhong guo\t100\n这个\tzhe ge\t90\n").unwrap();
        let dict = BinaryDict::from_entries(entries);
        let hits: Vec<_> = dict
            .lookup_candidates("zg")
            .into_iter()
            .map(|e| e.word.as_str())
            .collect();
        assert!(hits.contains(&"中国"));
        assert!(hits.contains(&"这个"));
    }

    #[test]
    fn lookup_prefix_wode_in_wodemaya() {
        let entries = parse_tsv("我的\two de\t100\n我\two\t90\n").unwrap();
        let dict = BinaryDict::from_entries(entries);
        let hits: Vec<_> = dict
            .lookup_prefix("wodemaya")
            .into_iter()
            .map(|e| e.word.as_str())
            .collect();
        assert!(hits.contains(&"我的"));
        assert!(hits.contains(&"我"));
        assert!(dict.lookup_candidates("wodemaya").is_empty());
    }

    #[test]
    fn lookup_prefix_single_letter_is_not_everything() {
        let entries = parse_tsv("中国\tzhong guo\t100\n这个\tzhe ge\t90\n造词\tzao ci\t1\n").unwrap();
        let dict = BinaryDict::from_entries(entries);
        assert!(
            dict.lookup_prefix("z").is_empty(),
            "z must not dump every z* word: {:?}",
            dict.lookup_prefix("z")
                .iter()
                .map(|e| e.word.as_str())
                .collect::<Vec<_>>()
        );
        let hits: Vec<_> = dict
            .lookup_prefix("zhongguo")
            .into_iter()
            .map(|e| e.word.as_str())
            .collect();
        assert!(hits.contains(&"中国"));
    }
}
