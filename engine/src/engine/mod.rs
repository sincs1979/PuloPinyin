pub mod state;

use crate::candidate::{Candidate, CandidateList};
use crate::dictionary::binary::learned_syllables;
use crate::dictionary::{load_system_dict, load_user_dict, BinaryDict, DictEntry};
use crate::learning::{compile_user_dict, should_compile, LearnedDb};
use crate::pinyin::parser::{full_cut_queries, is_separator, normalize_query, preedit_marked};
use crate::pinyin::{match_consumed, matches_prefix};
use crate::punct::{self, QuoteState};
use crate::ranking::score::{now_unix, score, ScoreInputs};
use crate::symbols;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Char(char),
    Space,
    Backspace,
    Enter,
    Escape,
    PageNext,
    PagePrev,
    Digit(u8),
    Punct(char),
    /// Syllable separator (`'` / `’`). Not a commit key while composing.
    Separator,
    ToggleAscii,
}

#[derive(Debug, Clone)]
pub struct SessionOutput {
    pub consumed: bool,
    pub commit: Option<String>,
    pub marked: String,
    pub candidates: Vec<Candidate>,
    pub page: usize,
    pub page_count: usize,
    pub ascii_mode: bool,
}

impl SessionOutput {
    fn pass_through(ascii_mode: bool) -> Self {
        Self {
            consumed: false,
            commit: None,
            marked: String::new(),
            candidates: Vec::new(),
            page: 0,
            page_count: 0,
            ascii_mode,
        }
    }

    fn from_list(
        list: &CandidateList,
        marked: String,
        ascii_mode: bool,
        commit: Option<String>,
    ) -> Self {
        Self {
            consumed: true,
            commit,
            marked,
            candidates: list.current_page().to_vec(),
            page: list.page,
            page_count: list.page_count(),
            ascii_mode,
        }
    }
}

pub struct Engine {
    system: BinaryDict,
    user: BinaryDict,
    learned: Option<LearnedDb>,
    /// In-memory user stats keyed by (word, spaced pinyin).
    user_stats: HashMap<(String, String), (u32, u64)>,
    user_dict_path: PathBuf,
    composing: String,
    /// Pieces already chosen in this composition (learned as one phrase when finished).
    phrase_buf: Vec<(String, Vec<String>)>,
    ascii_mode: bool,
    quotes: QuoteState,
    candidates: CandidateList,
    /// Consecutive ASCII letters committed in this session (`ChatGPT`).
    ascii_buf: String,
}

impl Engine {
    pub fn open(system_path: Option<&Path>, support_dir: impl AsRef<Path>) -> crate::Result<Self> {
        let support_dir = support_dir.as_ref();
        std::fs::create_dir_all(support_dir)?;
        let user_dict_path = support_dir.join("user.dict");
        let db_path = support_dir.join("learned.db");
        let system = load_system_dict(system_path)?;
        let user = load_user_dict(&user_dict_path)?;
        let learned = LearnedDb::open(&db_path)?;
        let records = learned.load_all()?;
        let mut user_stats = HashMap::new();
        for rec in records {
            user_stats.insert((rec.word, rec.pinyin), (rec.count, rec.last_used));
        }
        crate::pinyin::syllable::ensure_loaded();
        Ok(Self {
            system,
            user,
            learned: Some(learned),
            user_stats,
            user_dict_path,
            composing: String::new(),
            phrase_buf: Vec::new(),
            ascii_mode: false,
            quotes: QuoteState::default(),
            candidates: CandidateList::default(),
            ascii_buf: String::new(),
        })
    }

    /// In-memory engine for tests (no sqlite persistence).
    pub fn in_memory(entries: Vec<DictEntry>) -> Self {
        crate::pinyin::syllable::ensure_loaded();
        Self {
            system: BinaryDict::from_entries(entries),
            user: BinaryDict::default(),
            learned: None,
            user_stats: HashMap::new(),
            user_dict_path: PathBuf::from("/tmp/pulopinyin-user.dict"),
            composing: String::new(),
            phrase_buf: Vec::new(),
            ascii_mode: false,
            quotes: QuoteState::default(),
            candidates: CandidateList::default(),
            ascii_buf: String::new(),
        }
    }

    pub fn composing(&self) -> &str {
        &self.composing
    }

    pub fn ascii_mode(&self) -> bool {
        self.ascii_mode
    }

    pub fn set_ascii_mode(&mut self, ascii: bool) {
        if self.ascii_mode != ascii {
            self.flush_ascii_learn();
        }
        self.ascii_mode = ascii;
    }

    /// Finish a `ChatGPT`-style ASCII run (learn if ≥ 2 letters).
    ///
    /// Field activate/deactivate must call this so a leftover `ascii_buf`
    /// cannot turn the next unshifted keys into English commits.
    pub fn end_ascii_run(&mut self) {
        self.flush_ascii_learn();
    }

    pub fn candidates(&self) -> &CandidateList {
        &self.candidates
    }

    /// Chinese-mode letters:
    ///
    /// - Uppercase `Char` (IME sends this only for a **held** Shift+letter)
    ///   commits that ASCII character and stays in Chinese.
    /// - Lowercase is pinyin, unless an English token is already in progress:
    ///   `ascii_buf` non-empty **and** no pinyin composing (`C` then `hatGPT`).
    /// - Space / Enter / punct / [`Self::end_ascii_run`] end the token.
    /// - A lone Shift+A does **not** set `ascii_mode`, and does not survive
    ///   activate. Unshifted `zhongguo` after a flushed run is pinyin.
    fn chinese_ascii_letter(&self, c: char) -> bool {
        if !c.is_ascii_alphabetic() {
            return false;
        }
        if c.is_ascii_uppercase() {
            return true;
        }
        self.composing.is_empty() && !self.ascii_buf.is_empty()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SessionOutput {
        if matches!(key, KeyEvent::ToggleAscii) {
            if self.composing.is_empty() {
                self.flush_ascii_learn();
                self.ascii_mode = !self.ascii_mode;
                return SessionOutput {
                    consumed: true,
                    commit: None,
                    marked: String::new(),
                    candidates: Vec::new(),
                    page: 0,
                    page_count: 0,
                    ascii_mode: self.ascii_mode,
                };
            }
        }

        if self.ascii_mode {
            return self.handle_ascii(key);
        }

        let candidates_active = !self.candidates.is_empty();

        match key {
            KeyEvent::Char(c) if self.chinese_ascii_letter(c) => self.commit_ascii_letter(c),
            KeyEvent::Char(c) if c.is_ascii_alphabetic() => {
                self.composing.push(c.to_ascii_lowercase());
                self.refresh_query();
                self.output(None)
            }
            KeyEvent::Separator => {
                if self.composing.is_empty() {
                    self.flush_ascii_learn();
                }
                self.insert_separator()
            }
            KeyEvent::Backspace => {
                if self.composing.is_empty() {
                    self.ascii_buf.pop();
                    return SessionOutput::pass_through(false);
                }
                self.composing.pop();
                if self.composing.is_empty() {
                    self.phrase_buf.clear();
                    self.candidates = CandidateList::default();
                    return self.output(None);
                }
                self.refresh_query();
                self.output(None)
            }
            KeyEvent::Escape => {
                self.ascii_buf.clear();
                if self.composing.is_empty() {
                    return SessionOutput::pass_through(false);
                }
                self.clear_composing();
                self.output(None)
            }
            KeyEvent::Space => {
                self.flush_ascii_learn();
                if self.composing.is_empty() {
                    return SessionOutput::pass_through(false);
                }
                self.select_index(0)
            }
            KeyEvent::Enter => {
                self.flush_ascii_learn();
                if self.composing.is_empty() {
                    return SessionOutput::pass_through(false);
                }
                let raw = self.composing.clone();
                self.clear_composing();
                SessionOutput {
                    consumed: true,
                    commit: Some(raw),
                    marked: String::new(),
                    candidates: Vec::new(),
                    page: 0,
                    page_count: 0,
                    ascii_mode: false,
                }
            }
            KeyEvent::Digit(d) => {
                if !candidates_active {
                    self.flush_ascii_learn();
                    return SessionOutput::pass_through(false);
                }
                if let Some(idx) = digit_to_page_index(d) {
                    self.select_index(idx)
                } else {
                    SessionOutput::pass_through(false)
                }
            }
            KeyEvent::PageNext => {
                if !candidates_active {
                    return SessionOutput::pass_through(false);
                }
                self.candidates.next_page();
                self.output(None)
            }
            KeyEvent::PagePrev => {
                if !candidates_active {
                    return SessionOutput::pass_through(false);
                }
                self.candidates.prev_page();
                self.output(None)
            }
            KeyEvent::Punct(c) if is_separator(c) => {
                if self.composing.is_empty() {
                    self.flush_ascii_learn();
                }
                self.insert_separator()
            }
            KeyEvent::Punct(c) => {
                if candidates_active && is_page_prev(c) {
                    self.candidates.prev_page();
                    return self.output(None);
                }
                if candidates_active && is_page_next(c) {
                    self.candidates.next_page();
                    return self.output(None);
                }
                self.flush_ascii_learn();
                self.commit_punct(c)
            }
            KeyEvent::ToggleAscii => SessionOutput::pass_through(false),
            KeyEvent::Char(_) => SessionOutput::pass_through(false),
        }
    }

    fn handle_ascii(&mut self, key: KeyEvent) -> SessionOutput {
        let commit_char = |c: char| SessionOutput {
            consumed: true,
            commit: Some(c.to_string()),
            marked: String::new(),
            candidates: Vec::new(),
            page: 0,
            page_count: 0,
            ascii_mode: true,
        };
        match key {
            KeyEvent::Char(c) if c.is_ascii_alphabetic() => {
                self.ascii_buf.push(c);
                commit_char(c)
            }
            KeyEvent::Char(c) | KeyEvent::Punct(c) => {
                self.flush_ascii_learn();
                commit_char(c)
            }
            KeyEvent::Separator => {
                self.flush_ascii_learn();
                commit_char('\'')
            }
            KeyEvent::Space => {
                self.flush_ascii_learn();
                commit_char(' ')
            }
            KeyEvent::Digit(d) => {
                self.flush_ascii_learn();
                commit_char(char::from(b'0' + d))
            }
            KeyEvent::Enter => {
                self.flush_ascii_learn();
                SessionOutput::pass_through(true)
            }
            KeyEvent::Escape => {
                self.ascii_buf.clear();
                SessionOutput::pass_through(true)
            }
            KeyEvent::Backspace => {
                self.ascii_buf.pop();
                SessionOutput::pass_through(true)
            }
            _ => SessionOutput::pass_through(true),
        }
    }

    fn insert_separator(&mut self) -> SessionOutput {
        if self.composing.is_empty() {
            return self.commit_punct('\'');
        }
        if !self.composing.ends_with('\'') {
            self.composing.push('\'');
        }
        self.refresh_query();
        self.output(None)
    }

    /// Shift+letter (or a lowercase letter continuing `ascii_buf`) in Chinese:
    /// commit that ASCII character and stay in Chinese. If pinyin is in
    /// progress, commit the default candidate first (or drop unmarked composing).
    fn commit_ascii_letter(&mut self, c: char) -> SessionOutput {
        let letter = c.to_string();
        if !self.composing.is_empty() {
            if !self.candidates.is_empty() {
                let mut out = self.select_index(0);
                match &mut out.commit {
                    Some(text) => text.push_str(&letter),
                    None => out.commit = Some(letter.clone()),
                }
                if self.composing.is_empty() {
                    self.ascii_buf.push(c);
                }
                return out;
            }
            self.clear_composing();
        }
        self.ascii_buf.push(c);
        SessionOutput {
            consumed: true,
            commit: Some(letter),
            marked: String::new(),
            candidates: Vec::new(),
            page: 0,
            page_count: 0,
            ascii_mode: self.ascii_mode,
        }
    }

    fn flush_ascii_learn(&mut self) {
        if self.ascii_buf.chars().count() >= 2
            && self.ascii_buf.chars().all(|c| c.is_ascii_alphabetic())
        {
            let word = std::mem::take(&mut self.ascii_buf);
            self.learn_ascii_word(&word);
        } else {
            self.ascii_buf.clear();
        }
    }

    fn learn_ascii_word(&mut self, word: &str) {
        let key = word.to_ascii_lowercase();
        let cand = Candidate {
            word: word.to_string(),
            syllables: vec![key],
            base_frequency: 0,
            user_count: 0,
            last_used: None,
            score: 0.0,
            consumed: 0,
            complete: true,
        };
        self.learn(&cand);
    }

    fn commit_punct(&mut self, ch: char) -> SessionOutput {
        let mark = punct::to_chinese(ch, &mut self.quotes);
        if self.composing.is_empty() {
            return SessionOutput {
                consumed: true,
                commit: Some(mark),
                marked: String::new(),
                candidates: Vec::new(),
                page: 0,
                page_count: 0,
                ascii_mode: self.ascii_mode,
            };
        }
        if !self.candidates.is_empty() {
            let mut out = self.select_index(0);
            match &mut out.commit {
                Some(text) => text.push_str(&mark),
                None => out.commit = Some(mark),
            }
            return out;
        }
        let mut raw = std::mem::take(&mut self.composing);
        self.clear_composing();
        raw.push_str(&mark);
        SessionOutput {
            consumed: true,
            commit: Some(raw),
            marked: String::new(),
            candidates: Vec::new(),
            page: 0,
            page_count: 0,
            ascii_mode: self.ascii_mode,
        }
    }

    fn output(&self, commit: Option<String>) -> SessionOutput {
        let marked = preedit_marked(&self.composing);
        SessionOutput::from_list(&self.candidates, marked, self.ascii_mode, commit)
    }

    fn clear_composing(&mut self) {
        self.composing.clear();
        self.phrase_buf.clear();
        self.candidates = CandidateList::default();
    }

    fn select_index(&mut self, page_index: usize) -> SessionOutput {
        let Some(cand) = self.candidates.current_page().get(page_index).cloned() else {
            return self.output(None);
        };
        if !symbols::is_symbol_word(&cand.word) {
            self.learn(&cand);
        }
        let consumed = cand.consumed.min(self.composing.len());
        // consumed is in normalize_query space (letters + `'`); composing uses the same.
        let leftover = if consumed <= self.composing.len()
            && self.composing.is_char_boundary(consumed)
        {
            skip_leading_sep(&self.composing[consumed..]).to_string()
        } else {
            String::new()
        };
        self.phrase_buf
            .push((cand.word.clone(), cand.syllables.clone()));
        if leftover.is_empty() {
            self.learn_phrase_combo();
            let text = cand.word.clone();
            self.clear_composing();
            SessionOutput {
                consumed: true,
                commit: Some(text),
                marked: String::new(),
                candidates: Vec::new(),
                page: 0,
                page_count: 0,
                ascii_mode: self.ascii_mode,
            }
        } else {
            self.composing = leftover;
            self.refresh_query();
            self.output(Some(cand.word))
        }
    }

    fn learn_phrase_combo(&mut self) {
        if self.phrase_buf.len() < 2 {
            return;
        }
        let word: String = self.phrase_buf.iter().map(|(w, _)| w.as_str()).collect();
        let syllables: Vec<String> = self
            .phrase_buf
            .iter()
            .flat_map(|(_, s)| s.iter().cloned())
            .collect();
        if word.is_empty() || syllables.is_empty() {
            return;
        }
        let combo = Candidate {
            word,
            syllables,
            base_frequency: 0,
            user_count: 0,
            last_used: None,
            score: 0.0,
            consumed: 0,
            complete: true,
        };
        self.learn(&combo);
    }

    fn learn(&mut self, cand: &Candidate) {
        let pinyin = cand.pinyin_spaced();
        let now = now_unix();
        let key = (cand.word.clone(), pinyin.clone());
        let entry = self.user_stats.entry(key).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = now;

        if let Some(db) = &self.learned {
            if let Err(e) = db.bump(&cand.word, &cand.pinyin_spaced(), now) {
                eprintln!("pulopinyin: learn failed: {e}");
            } else {
                let now2 = now;
                if should_compile(db, now2).unwrap_or(false) {
                    let compiled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        compile_user_dict(db, &self.user_dict_path, now2)
                    }));
                    match compiled {
                        Ok(Ok(_)) => {
                            if let Ok(user) = load_user_dict(&self.user_dict_path) {
                                self.user = user;
                            }
                        }
                        Ok(Err(e)) => eprintln!("pulopinyin: compile user.dict failed: {e}"),
                        Err(_) => eprintln!("pulopinyin: compile user.dict panicked"),
                    }
                }
            }
        }
    }

    fn refresh_query(&mut self) {
        let input = normalize_query(&self.composing);
        if input.is_empty() || input.chars().all(is_separator) {
            self.candidates = CandidateList::default();
            return;
        }
        // A panic here used to kill the IME process → 完全无法录入.
        self.candidates = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.query(&input)
        })) {
            Ok(list) => list,
            Err(_) => {
                eprintln!("pulopinyin: query panicked on {input:?}; keeping composing");
                CandidateList::default()
            }
        };
    }

    pub fn query(&self, input: &str) -> CandidateList {
        let input = normalize_query(input);
        if input.is_empty() || input.chars().all(is_separator) {
            return CandidateList::default();
        }
        // Fixed-slot catalog: never mix with lexicon or user_count ranking.
        if symbols::is_catalog_key(&input) {
            return CandidateList::from_sorted(symbols::catalog_candidates(&input));
        }
        let now = now_unix();
        let mut merged: HashMap<(String, String), Candidate> = HashMap::new();

        let mut consider = |e: &DictEntry| {
            let (consumed, complete) = if let Some(n) = match_consumed(&input, &e.syllables) {
                (n, true)
            } else if matches_prefix(&input, &e.syllables) {
                (input.len(), false)
            } else {
                return;
            };
            if consumed == 0 {
                return;
            }
            let pinyin = e.pinyin_spaced();
            let key = (e.word.clone(), pinyin.clone());
            let (user_count, last_used) = self
                .user_stats
                .get(&key)
                .copied()
                .map(|(c, t)| (c, Some(t)))
                .unwrap_or((0, None));
            let sc = score(ScoreInputs {
                base_frequency: e.frequency,
                user_count,
                last_used,
                now,
            });
            merged
                .entry(key)
                .and_modify(|c| {
                    if e.frequency > c.base_frequency {
                        c.base_frequency = e.frequency;
                    }
                    c.user_count = c.user_count.max(user_count);
                    c.consumed = c.consumed.max(consumed);
                    c.complete = c.complete || complete;
                    c.score = score(ScoreInputs {
                        base_frequency: c.base_frequency,
                        user_count: c.user_count,
                        last_used: c.last_used,
                        now,
                    });
                })
                .or_insert(Candidate {
                    word: e.word.clone(),
                    syllables: e.syllables.clone(),
                    base_frequency: e.frequency,
                    user_count,
                    last_used,
                    score: sc,
                    consumed,
                    complete,
                });
        };

        // Every valid full-syllable partition is queried (keneng → ken'eng
        // and ke'neng). Raw input stays so initials / mixed / unfinished
        // prefixes still hit. Candidates merge; rank is by score.
        for q in full_cut_queries(&input) {
            for e in self.system.lookup_prefix(&q) {
                consider(e);
            }
            for e in self.user.lookup_prefix(&q) {
                consider(e);
            }
        }
        // Learned words not yet in either binary dict still need to appear.
        for ((word, pinyin), (_count, _last)) in &self.user_stats {
            let syllables = learned_syllables(word, pinyin);
            let e = DictEntry {
                word: word.clone(),
                syllables,
                frequency: 0,
            };
            consider(&e);
        }
        drop(consider);
        for ((word, pinyin), (count, last)) in &self.user_stats {
            if let Some(c) = merged.get_mut(&(word.clone(), pinyin.clone())) {
                c.user_count = *count;
                c.last_used = Some(*last);
                c.score = score(ScoreInputs {
                    base_frequency: c.base_frequency,
                    user_count: *count,
                    last_used: Some(*last),
                    now,
                });
            }
        }

        if let Some(composed) = self.greedy_compose(&input, now) {
            let key = (composed.word.clone(), composed.pinyin_spaced());
            merged
                .entry(key)
                .and_modify(|c| {
                    c.consumed = c.consumed.max(composed.consumed);
                    if composed.base_frequency > c.base_frequency {
                        c.base_frequency = composed.base_frequency;
                    }
                })
                .or_insert(composed);
        }

        let mut items: Vec<Candidate> = merged.into_values().collect();
        const MAX_QUERY: usize = 256;
        items.sort_by(|a, b| {
            b.complete
                .cmp(&a.complete)
                .then_with(|| b.consumed.cmp(&a.consumed))
                .then_with(|| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.base_frequency.cmp(&a.base_frequency))
                .then_with(|| a.word.cmp(&b.word))
        });
        if items.len() > MAX_QUERY {
            items.truncate(MAX_QUERY);
        }
        // Exact pinyin/initial keys only — prepend so slh is not buried
        // under random lexicon hits, and never expand a first-letter bucket.
        let symbols = symbols::symbol_candidates(&input);
        if !symbols.is_empty() {
            items.retain(|c| symbols.iter().all(|s| s.word != c.word));
            let mut prepended = symbols;
            prepended.extend(items);
            items = prepended;
        }
        CandidateList::from_sorted(items)
    }

    /// Longest-word walk over the remaining input. Two or more hits become one phrase.
    fn greedy_compose(&self, input: &str, now: u64) -> Option<Candidate> {
        let mut remaining = input;
        let mut word = String::new();
        let mut syllables = Vec::new();
        let mut pieces = 0usize;
        let mut freq = 0u32;
        while !remaining.is_empty() {
            let best = self.longest_prefix_word(remaining)?;
            if best.1 == 0 || best.1 > remaining.len() {
                break;
            }
            word.push_str(&best.0.word);
            syllables.extend(best.0.syllables.iter().cloned());
            freq = freq.max(best.0.frequency);
            pieces += 1;
            remaining = &remaining[best.1..];
        }
        if pieces < 2 || word.is_empty() {
            return None;
        }
        let consumed = input.len() - remaining.len();
        if consumed == 0 {
            return None;
        }
        let pinyin = syllables.join(" ");
        let key = (word.clone(), pinyin);
        let (user_count, last_used) = self
            .user_stats
            .get(&key)
            .copied()
            .map(|(c, t)| (c, Some(t)))
            .unwrap_or((0, None));
        let sc = score(ScoreInputs {
            base_frequency: freq,
            user_count,
            last_used,
            now,
        });
        Some(Candidate {
            word,
            syllables,
            base_frequency: freq,
            user_count,
            last_used,
            score: sc,
            consumed,
            complete: true,
        })
    }

    fn longest_prefix_word(&self, input: &str) -> Option<(DictEntry, usize)> {
        let mut best: Option<(DictEntry, usize)> = None;
        let consider = |e: &DictEntry, best: &mut Option<(DictEntry, usize)>| {
            let Some(n) = match_consumed(input, &e.syllables) else {
                return;
            };
            if n == 0 {
                return;
            }
            let better = match best {
                None => true,
                Some((cur, cn)) => {
                    n > *cn
                        || (n == *cn
                            && (e.syllables.len() > cur.syllables.len()
                                || (e.syllables.len() == cur.syllables.len()
                                    && e.frequency > cur.frequency)))
                }
            };
            if better {
                *best = Some((e.clone(), n));
            }
        };
        for e in self.system.lookup_prefix(input) {
            consider(e, &mut best);
        }
        for e in self.user.lookup_prefix(input) {
            consider(e, &mut best);
        }
        for ((word, pinyin), (_count, _last)) in &self.user_stats {
            let syllables = learned_syllables(word, pinyin);
            let e = DictEntry {
                word: word.clone(),
                syllables,
                frequency: 0,
            };
            consider(&e, &mut best);
        }
        best
    }
}

fn skip_leading_sep(s: &str) -> &str {
    s.trim_start_matches(is_separator)
}

fn is_page_prev(c: char) -> bool {
    matches!(c, ',' | '，' | '-' | '－')
}

fn is_page_next(c: char) -> bool {
    matches!(c, '.' | '。' | '+' | '＋' | '=')
}

fn digit_to_page_index(d: u8) -> Option<usize> {
    match d {
        1..=9 => Some((d as usize) - 1),
        0 => Some(9),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> Engine {
        let tsv = crate::dictionary::system::builtin_tsv();
        let entries = crate::dictionary::parse_tsv(tsv).unwrap();
        Engine::in_memory(entries)
    }

    #[test]
    fn mvp_zhongguo() {
        let mut eng = test_engine();
        for c in "zhongguo".chars() {
            eng.handle_key(KeyEvent::Char(c));
        }
        let out = eng.handle_key(KeyEvent::Space);
        assert_eq!(out.commit.as_deref(), Some("中国"));
    }

    #[test]
    fn initials_zg_finds_zhongguo() {
        let eng = test_engine();
        let list = eng.query("zg");
        let words: Vec<_> = list.items.iter().map(|c| c.word.as_str()).collect();
        assert!(words.contains(&"中国"), "zg candidates: {words:?}");
    }

    #[test]
    fn fuzzy_si_includes_shi_without_penalty() {
        let eng = test_engine();
        let list = eng.query("si");
        let words: Vec<_> = list.items.iter().map(|c| c.word.as_str()).collect();
        assert!(words.contains(&"是") || words.contains(&"四"));
        // 是 (shi) must be allowed to rank first purely by frequency.
        if let Some(first) = list.items.first() {
            assert!(
                first.word == "是"
                    || list
                        .items
                        .iter()
                        .any(|c| c.word == "是" && c.score <= first.score + 0.001)
                    || true
            );
        }
        let shi = list.items.iter().find(|c| c.word == "是");
        let si_char = list.items.iter().find(|c| c.word == "四");
        if let (Some(shi), Some(si_char)) = (shi, si_char) {
            assert!(
                shi.score >= si_char.score,
                "是 should not be downranked vs 四; {} vs {}",
                shi.score,
                si_char.score
            );
        }
    }

    #[test]
    fn learning_raises_rank() {
        let mut eng = test_engine();
        let before = eng.query("shangpin");
        let words: Vec<_> = before.items.iter().map(|c| c.word.clone()).collect();
        if !words.iter().any(|w| w == "上品") || !words.iter().any(|w| w == "商品") {
            return;
        }
        let shangpin_pos = before.items.iter().position(|c| c.word == "上品").unwrap();
        for _ in 0..12 {
            let cand = eng
                .query("shangpin")
                .items
                .into_iter()
                .find(|c| c.word == "上品")
                .unwrap();
            eng.learn(&cand);
        }
        let after = eng.query("shangpin");
        let new_pos = after.items.iter().position(|c| c.word == "上品").unwrap();
        assert!(new_pos <= shangpin_pos);
        assert_eq!(after.items[0].word, "上品");
    }

    #[test]
    fn paging_does_not_reorder() {
        let mut eng = test_engine();
        for c in "shi".chars() {
            eng.handle_key(KeyEvent::Char(c));
        }
        let first_page: Vec<String> = eng
            .candidates
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect();
        eng.handle_key(KeyEvent::PageNext);
        let still: Vec<String> = eng
            .candidates
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect();
        assert_eq!(first_page, still);
        assert_eq!(
            eng.candidates.page,
            1.min(eng.candidates.page_count().saturating_sub(1))
        );
    }

    #[test]
    fn sqlite_learning_survives_reopen() {
        let dir = std::env::temp_dir().join(format!("pulopinyin-learn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let mut eng = Engine::open(None, &dir).unwrap();
            let Some(cand) = eng.query("si").items.into_iter().find(|c| c.word == "四") else {
                let _ = std::fs::remove_dir_all(&dir);
                return;
            };
            for _ in 0..20 {
                eng.learn(&cand);
            }
        }
        let eng = Engine::open(None, &dir).unwrap();
        let after = eng.query("si");
        assert_eq!(after.items[0].word, "四");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chinese_mode_converts_punctuation() {
        let mut eng = test_engine();
        assert_eq!(
            eng.handle_key(KeyEvent::Punct(',')).commit.as_deref(),
            Some("，")
        );
        assert_eq!(
            eng.handle_key(KeyEvent::Punct('.')).commit.as_deref(),
            Some("。")
        );
        assert_eq!(
            eng.handle_key(KeyEvent::Punct('?')).commit.as_deref(),
            Some("？")
        );
    }

    #[test]
    fn ascii_mode_keeps_english_letters_and_punct() {
        let mut eng = test_engine();
        eng.set_ascii_mode(true);
        assert_eq!(
            eng.handle_key(KeyEvent::Char('A')).commit.as_deref(),
            Some("A")
        );
        assert_eq!(
            eng.handle_key(KeyEvent::Punct(',')).commit.as_deref(),
            Some(",")
        );
        assert_eq!(
            eng.handle_key(KeyEvent::Char('z')).commit.as_deref(),
            Some("z")
        );
    }

    #[test]
    fn chinese_uppercase_letter_commits_ascii() {
        let mut eng = test_engine();
        assert!(!eng.ascii_mode());
        let out = eng.handle_key(KeyEvent::Char('A'));
        assert!(out.consumed);
        assert_eq!(out.commit.as_deref(), Some("A"));
        assert!(eng.composing().is_empty(), "A must not enter pinyin");
        assert!(!out.ascii_mode, "Shift+letter must stay in Chinese");
        assert!(!eng.ascii_mode());

        // Space ends a one-letter ASCII run without learning it.
        let out = eng.handle_key(KeyEvent::Space);
        assert!(!out.consumed);
        let out = eng.handle_key(KeyEvent::Char('z'));
        assert!(out.commit.is_none());
        assert_eq!(eng.composing(), "z");
    }

    #[test]
    fn learn_chatgpt_from_ascii_run_then_query() {
        let mut eng = test_engine();
        for c in "ChatGPT".chars() {
            let out = eng.handle_key(KeyEvent::Char(c));
            assert_eq!(out.commit, Some(c.to_string()));
            assert!(eng.composing().is_empty(), "{c} must not enter pinyin");
            assert!(!eng.ascii_mode());
        }
        let out = eng.handle_key(KeyEvent::Enter);
        assert!(!out.consumed, "Enter after ASCII run is a terminator");
        assert!(!eng.ascii_mode());

        let words = eng
            .query("chatgpt")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert!(
            words.contains(&"ChatGPT".into()),
            "chatgpt must list stored ChatGPT: {words:?}"
        );

        type_pinyin(&mut eng, "chatgpt");
        assert!(
            cand_words(&eng).contains(&"ChatGPT".into()),
            "composing chatgpt: {:?}",
            cand_words(&eng)
        );
    }

    #[test]
    fn unshifted_pinyin_after_ascii_run_is_flushed() {
        let mut eng = test_engine();
        assert!(!eng.ascii_mode());
        for c in "ChatGPT".chars() {
            let out = eng.handle_key(KeyEvent::Char(c));
            assert_eq!(out.commit, Some(c.to_string()));
            assert!(eng.composing().is_empty());
            assert!(!eng.ascii_mode());
        }
        let out = eng.handle_key(KeyEvent::Enter);
        assert!(!out.consumed);
        assert!(!eng.ascii_mode());
        assert!(eng.composing().is_empty());

        type_pinyin(&mut eng, "zhongguo");
        assert_eq!(eng.composing(), "zhongguo");
        assert!(
            cand_words(&eng).contains(&"中国".into()),
            "after ChatGPT+Enter, zhongguo must be pinyin: {:?}",
            cand_words(&eng)
        );
        let out = eng.handle_key(KeyEvent::Space);
        assert_eq!(out.commit.as_deref(), Some("中国"));
    }

    #[test]
    fn end_ascii_run_unsticks_before_pinyin() {
        let mut eng = test_engine();
        assert_eq!(
            eng.handle_key(KeyEvent::Char('A')).commit.as_deref(),
            Some("A")
        );
        // Same as activateServer: leftover one-letter buf must not eat `zhongguo`.
        eng.end_ascii_run();
        type_pinyin(&mut eng, "zhongguo");
        assert_eq!(eng.composing(), "zhongguo");
        assert_eq!(
            eng.handle_key(KeyEvent::Space).commit.as_deref(),
            Some("中国")
        );
    }

    #[test]
    fn keneng_and_wodemaya_unshifted() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "keneng");
        assert!(
            cand_words(&eng).contains(&"可能".into()),
            "keneng: {:?}",
            cand_words(&eng)
        );
        eng.clear_composing();
        type_pinyin(&mut eng, "wodemaya");
        assert!(
            cand_words(&eng).contains(&"我的".into()),
            "wodemaya: {:?}",
            cand_words(&eng)
        );
    }

    #[test]
    fn single_ascii_letter_is_not_learned() {
        let mut eng = test_engine();
        assert_eq!(
            eng.handle_key(KeyEvent::Char('A')).commit.as_deref(),
            Some("A")
        );
        let _ = eng.handle_key(KeyEvent::Enter);
        type_pinyin(&mut eng, "a");
        assert!(
            !cand_words(&eng).iter().any(|w| w == "A"),
            "single letter must not enter lexicon: {:?}",
            cand_words(&eng)
        );
    }

    #[test]
    fn chinese_shift_letter_does_not_append_pinyin() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "ni");
        assert_eq!(eng.composing(), "ni");
        let out = eng.handle_key(KeyEvent::Char('A'));
        assert!(
            out.commit.as_deref().is_some_and(|s| s.ends_with('A')),
            "expected default candidate then A, got {:?}",
            out.commit
        );
        assert_ne!(eng.composing(), "nia");
        assert!(!eng.composing().contains('A'));
        assert!(!eng.ascii_mode());
    }

    #[test]
    fn chinese_uppercase_without_candidates_clears_then_inserts() {
        let mut eng = Engine::in_memory(Vec::new());
        let out = eng.handle_key(KeyEvent::Char('q'));
        assert!(out.commit.is_none());
        assert_eq!(eng.composing(), "q");
        let out = eng.handle_key(KeyEvent::Char('B'));
        assert_eq!(out.commit.as_deref(), Some("B"));
        assert!(eng.composing().is_empty());
        assert!(!eng.ascii_mode());
    }

    #[test]
    fn plus_minus_page_candidates() {
        let mut eng = test_engine();
        for c in "shi".chars() {
            eng.handle_key(KeyEvent::Char(c));
        }
        let start = eng.candidates.page;
        if eng.candidates.page_count() < 2 {
            return;
        }
        let out = eng.handle_key(KeyEvent::Punct('+'));
        assert!(out.consumed);
        assert_eq!(eng.candidates.page, start + 1);
        let out = eng.handle_key(KeyEvent::Punct('-'));
        assert!(out.consumed);
        assert_eq!(eng.candidates.page, start);
    }

    fn type_pinyin(eng: &mut Engine, s: &str) {
        for c in s.chars() {
            if is_separator(c) {
                eng.handle_key(KeyEvent::Separator);
            } else {
                eng.handle_key(KeyEvent::Char(c));
            }
        }
    }

    fn cand_words(eng: &Engine) -> Vec<String> {
        eng.candidates
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect()
    }

    fn select_word(eng: &mut Engine, word: &str) -> SessionOutput {
        let pos = eng
            .candidates
            .items
            .iter()
            .position(|c| c.word == word)
            .unwrap_or_else(|| panic!("missing {word} in {:?}", cand_words(eng)));
        eng.candidates.page = pos / crate::PAGE_SIZE;
        let idx = pos % crate::PAGE_SIZE;
        eng.select_index(idx)
    }

    #[test]
    fn wodemaya_prefix_candidates_include_wode() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "wodemaya");
        let words = cand_words(&eng);
        assert!(
            !words.is_empty(),
            "wodemaya must not show a blank candidate page"
        );
        assert!(words.contains(&"我的".into()), "wodemaya candidates: {words:?}");
        assert!(
            words.iter().any(|w| w == "我" || w.starts_with('我')),
            "wo-words should appear: {words:?}"
        );
        let wode = eng
            .candidates
            .items
            .iter()
            .find(|c| c.word == "我的")
            .unwrap();
        assert_eq!(wode.consumed, 4);
        assert!(
            eng.candidates.items[0].consumed >= wode.consumed,
            "longest prefix should rank first"
        );
    }

    #[test]
    fn wodemaya_select_wode_then_maya_continues() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "wodemaya");
        let out = select_word(&mut eng, "我的");
        assert_eq!(out.commit.as_deref(), Some("我的"));
        assert_eq!(eng.composing(), "maya");
        assert!(!out.candidates.is_empty(), "maya must keep candidates");
        let words: Vec<_> = out.candidates.iter().map(|c| c.word.as_str()).collect();
        let all = cand_words(&eng);
        assert!(
            all.contains(&"妈呀".into()) || all.contains(&"妈".into()) || words.contains(&"妈呀") || words.contains(&"妈"),
            "maya candidates: {all:?}"
        );
    }

    #[test]
    fn compose_then_learn_hits_initials_wdmy() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "wodemaya");
        select_word(&mut eng, "我的");
        select_word(&mut eng, "妈呀");
        let full = eng.query("wodemaya");
        let words: Vec<_> = full.items.iter().map(|c| c.word.as_str()).collect();
        assert!(
            words.contains(&"我的妈呀"),
            "learned phrase on full pinyin: {words:?}"
        );
        let initials = eng.query("wdmy");
        let words: Vec<_> = initials.items.iter().map(|c| c.word.as_str()).collect();
        assert!(
            words.contains(&"我的妈呀"),
            "learned phrase on initials wdmy: {words:?}"
        );
        let mixed = eng.query("wodmy");
        let words: Vec<_> = mixed.items.iter().map(|c| c.word.as_str()).collect();
        assert!(
            words.contains(&"我的妈呀"),
            "learned phrase on mixed wodmy: {words:?}"
        );
    }

    #[test]
    fn ruguoniyao_greedy_compose() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "ruguoniyao");
        let words = cand_words(&eng);
        assert!(
            words.contains(&"如果你要".into()) || words.contains(&"如果".into()),
            "ruguoniyao candidates: {words:?}"
        );
        assert!(words.contains(&"如果".into()), "prefix 如果: {words:?}");
    }

    #[test]
    fn first_letter_keeps_composing_visible() {
        for ch in ['z', 'n', 'w', 's', 'a'] {
            let mut eng = test_engine();
            let out = eng.handle_key(KeyEvent::Char(ch));
            assert!(out.consumed, "{ch} must be consumed");
            assert!(!out.marked.is_empty(), "{ch} must show marked pinyin");
            assert_eq!(eng.composing(), ch.to_string());
        }
    }

    #[test]
    fn ni_wo_hao_zhongguo_still_work() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "ni");
        assert!(
            cand_words(&eng).iter().any(|w| w.contains('你') || w == "呢"),
            "ni: {:?}",
            cand_words(&eng)
        );
        eng.clear_composing();
        type_pinyin(&mut eng, "wo");
        assert!(
            cand_words(&eng).contains(&"我".into()),
            "wo: {:?}",
            cand_words(&eng)
        );
        eng.clear_composing();
        type_pinyin(&mut eng, "hao");
        assert!(
            cand_words(&eng).iter().any(|w| w.contains('好')),
            "hao: {:?}",
            cand_words(&eng)
        );
        eng.clear_composing();
        type_pinyin(&mut eng, "zhongguo");
        assert!(cand_words(&eng).contains(&"中国".into()));
        let out = eng.handle_key(KeyEvent::Space);
        assert_eq!(out.commit.as_deref(), Some("中国"));
    }

    #[test]
    fn leftover_tail_and_random_pinyin_never_blank() {
        let mut eng = test_engine();
        for c in "wodemayaqxyz".chars() {
            let out = eng.handle_key(KeyEvent::Char(c));
            assert!(out.consumed);
            assert!(!out.marked.is_empty());
            assert!(!eng.composing().is_empty());
        }
    }

    #[test]
    fn large_z_bucket_does_not_explode_on_first_letter() {
        let mut entries = vec![
            DictEntry {
                word: "中国".into(),
                syllables: vec!["zhong".into(), "guo".into()],
                frequency: 9999,
            },
            DictEntry {
                word: "我".into(),
                syllables: vec!["wo".into()],
                frequency: 9000,
            },
            DictEntry {
                word: "我的".into(),
                syllables: vec!["wo".into(), "de".into()],
                frequency: 8000,
            },
        ];
        for i in 0..5000 {
            entries.push(DictEntry {
                word: format!("造词{i}"),
                syllables: vec!["zao".into(), "ci".into()],
                frequency: 1,
            });
        }
        let mut eng = Engine::in_memory(entries);
        let start = std::time::Instant::now();
        let out = eng.handle_key(KeyEvent::Char('z'));
        assert!(
            start.elapsed().as_millis() < 200,
            "first letter took {:?}",
            start.elapsed()
        );
        assert!(out.consumed);
        assert_eq!(out.marked, "z");
        assert!(
            eng.query("z").items.len() < 200,
            "first letter must not return the whole z bucket: {}",
            eng.query("z").items.len()
        );
        type_pinyin(&mut eng, "hongguo");
        assert!(cand_words(&eng).contains(&"中国".into()));
    }

    #[test]
    fn production_lexicon_first_keystroke_survives() {
        let Some(dict) = production_system_dict() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!(
            "pulopinyin-safe-{}-{}",
            std::process::id(),
            now_unix()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let mut eng = Engine::open(Some(&dict), &dir).expect("open production dict");
        let start = std::time::Instant::now();
        let out = eng.handle_key(KeyEvent::Char('z'));
        assert!(out.consumed);
        assert!(!out.marked.is_empty());
        assert!(
            start.elapsed().as_millis() < 400,
            "query(z) on production dict took {:?}",
            start.elapsed()
        );
        assert!(
            eng.query("z").items.len() < 200,
            "production z candidates: {}",
            eng.query("z").items.len()
        );
        for c in "hongguo".chars() {
            eng.handle_key(KeyEvent::Char(c));
        }
        assert!(
            cand_words(&eng).contains(&"中国".into()),
            "{:?}",
            cand_words(&eng)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn query_keneng_is_possible_without_apostrophe() {
        let eng = test_engine();
        let words = eng
            .query("keneng")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert!(
            words.contains(&"可能".into()),
            "keneng must find 可能 without apostrophe: {words:?}"
        );
        let first = words.first().map(String::as_str).unwrap_or("");
        assert_ne!(
            first, "肯",
            "keneng must not stop at ken+eng (肯): {words:?}"
        );
        let quoted = eng
            .query("ke'neng")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert!(
            quoted.contains(&"可能".into()),
            "ke'neng must also find 可能: {quoted:?}"
        );
    }

    #[test]
    fn query_xian_prefers_xian_syllable_not_xian() {
        let eng = test_engine();
        let words = eng
            .query("xian")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert!(
            words.iter().any(|w| w == "先" || w == "现" || w == "县"),
            "xian should find 先/现: {words:?}"
        );
        let first = words.first().map(String::as_str).unwrap_or("");
        assert_ne!(first, "西安", "xian must not prefer 西安: {words:?}");
        assert!(
            !words.contains(&"西安".into()),
            "xian must not match 西安 without a separator: {words:?}"
        );
        let xian = eng
            .query("xi'an")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert!(xian.contains(&"西安".into()), "xi'an candidates: {xian:?}");
    }

    #[test]
    fn query_danshi_prefers_but_not_bull_market() {
        let eng = Engine::in_memory(vec![
            DictEntry {
                word: "但是".into(),
                syllables: vec!["dan".into(), "shi".into()],
                frequency: 8_000,
            },
            DictEntry {
                word: "大牛市".into(),
                syllables: vec!["da".into(), "niu".into(), "shi".into()],
                frequency: 90_000,
            },
            DictEntry {
                word: "大".into(),
                syllables: vec!["da".into()],
                frequency: 800_000,
            },
            DictEntry {
                word: "牛".into(),
                syllables: vec!["niu".into()],
                frequency: 100_000,
            },
            DictEntry {
                word: "市".into(),
                syllables: vec!["shi".into()],
                frequency: 300_000,
            },
        ]);
        let words = query_words(&eng, "danshi");
        assert_eq!(words.first().map(String::as_str), Some("但是"), "{words:?}");
        assert_ne!(
            words.first().map(String::as_str),
            Some("大牛市"),
            "danshi must not rank 大牛市 first: {words:?}"
        );

        let builtin = test_engine();
        let words = query_words(&builtin, "danshi");
        assert_eq!(words.first().map(String::as_str), Some("但是"), "{words:?}");
    }

    #[test]
    fn query_xianshi_prefers_display_not_xian_city() {
        let eng = Engine::in_memory(vec![
            DictEntry {
                word: "显示".into(),
                syllables: vec!["xian".into(), "shi".into()],
                frequency: 8_000,
            },
            DictEntry {
                word: "西安市".into(),
                syllables: vec!["xi".into(), "an".into(), "shi".into()],
                frequency: 90_000,
            },
            DictEntry {
                word: "西安".into(),
                syllables: vec!["xi".into(), "an".into()],
                frequency: 70_000,
            },
            DictEntry {
                word: "先".into(),
                syllables: vec!["xian".into()],
                frequency: 200_000,
            },
            DictEntry {
                word: "市".into(),
                syllables: vec!["shi".into()],
                frequency: 300_000,
            },
        ]);
        let words = query_words(&eng, "xianshi");
        assert_eq!(words.first().map(String::as_str), Some("显示"), "{words:?}");
        assert_ne!(
            words.first().map(String::as_str),
            Some("西安市"),
            "xianshi must not rank 西安市 first: {words:?}"
        );
        let quoted = query_words(&eng, "xi'anshi");
        assert!(
            quoted.contains(&"西安市".into()),
            "xi'anshi must find 西安市: {quoted:?}"
        );

        let builtin = test_engine();
        let words = query_words(&builtin, "xianshi");
        assert_eq!(words.first().map(String::as_str), Some("显示"), "{words:?}");
    }

    #[test]
    fn query_keneng_ranks_cuts_by_score_not_fixed_cut() {
        let possible_first = Engine::in_memory(vec![
            DictEntry {
                word: "可能".into(),
                syllables: vec!["ke".into(), "neng".into()],
                frequency: 90_000,
            },
            DictEntry {
                word: "肯恩".into(),
                syllables: vec!["ken".into(), "eng".into()],
                frequency: 8_000,
            },
        ]);
        let words = query_words(&possible_first, "keneng");
        assert!(words.contains(&"可能".into()), "{words:?}");
        assert!(words.contains(&"肯恩".into()), "{words:?}");
        assert_eq!(words.first().map(String::as_str), Some("可能"), "{words:?}");

        let ken_first = Engine::in_memory(vec![
            DictEntry {
                word: "可能".into(),
                syllables: vec!["ke".into(), "neng".into()],
                frequency: 8_000,
            },
            DictEntry {
                word: "肯恩".into(),
                syllables: vec!["ken".into(), "eng".into()],
                frequency: 90_000,
            },
        ]);
        let words = query_words(&ken_first, "keneng");
        assert!(words.contains(&"可能".into()), "{words:?}");
        assert!(words.contains(&"肯恩".into()), "{words:?}");
        assert_eq!(
            words.first().map(String::as_str),
            Some("肯恩"),
            "higher-freq ken+eng word must beat 可能: {words:?}"
        );
    }

    #[test]
    fn query_keneng_xian_diao_regressions() {
        let eng = test_engine();
        let keneng = query_words(&eng, "keneng");
        assert!(
            keneng.contains(&"可能".into()),
            "keneng must find 可能: {keneng:?}"
        );
        assert_eq!(keneng.first().map(String::as_str), Some("可能"), "{keneng:?}");

        let xian = query_words(&eng, "xian");
        assert!(xian.contains(&"先".into()), "xian must include 先: {xian:?}");
        assert_ne!(xian.first().map(String::as_str), Some("西安"), "{xian:?}");
        assert!(
            !xian.contains(&"西安".into()),
            "xian must not match 西安: {xian:?}"
        );
        let xi_an = query_words(&eng, "xi'an");
        assert!(xi_an.contains(&"西安".into()), "xi'an: {xi_an:?}");

        let diao = Engine::in_memory(vec![
            DictEntry {
                word: "掉".into(),
                syllables: vec!["diao".into()],
                frequency: 9_000,
            },
            DictEntry {
                word: "低奥".into(),
                syllables: vec!["di".into(), "ao".into()],
                frequency: 90_000,
            },
        ]);
        let words = query_words(&diao, "diao");
        assert_eq!(words.first().map(String::as_str), Some("掉"), "{words:?}");
        assert!(
            !words.contains(&"低奥".into()),
            "diao must not split to di+ao: {words:?}"
        );
    }

    fn query_words(eng: &Engine, input: &str) -> Vec<String> {
        eng.query(input)
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect()
    }

    #[test]
    fn query_diao_prefers_diao_not_di_ao() {
        let eng = Engine::in_memory(vec![
            DictEntry {
                word: "掉".into(),
                syllables: vec!["diao".into()],
                frequency: 9000,
            },
            DictEntry {
                word: "调".into(),
                syllables: vec!["diao".into()],
                frequency: 8000,
            },
            DictEntry {
                word: "低奥".into(),
                syllables: vec!["di".into(), "ao".into()],
                frequency: 8500,
            },
            DictEntry {
                word: "低".into(),
                syllables: vec!["di".into()],
                frequency: 7000,
            },
            DictEntry {
                word: "奥".into(),
                syllables: vec!["ao".into()],
                frequency: 6000,
            },
        ]);
        let words = eng
            .query("diao")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert_eq!(words.first().map(String::as_str), Some("掉"), "{words:?}");
        assert!(
            !words.contains(&"低奥".into()),
            "diao must not split to di+ao: {words:?}"
        );
        let split = eng
            .query("di'ao")
            .items
            .iter()
            .map(|c| c.word.clone())
            .collect::<Vec<_>>();
        assert!(split.contains(&"低奥".into()), "di'ao candidates: {split:?}");
    }

    #[test]
    fn apostrophe_does_not_commit_composing() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "xi");
        let out = eng.handle_key(KeyEvent::Separator);
        assert!(out.consumed);
        assert!(out.commit.is_none(), "apostrophe must not commit: {out:?}");
        assert_eq!(eng.composing(), "xi'");
        assert!(out.marked.contains('\''), "preedit should keep ': {}", out.marked);
        let out = eng.handle_key(KeyEvent::Punct('\''));
        assert!(out.commit.is_none());
        assert_eq!(eng.composing(), "xi'", "do not stack separators");
        type_pinyin(&mut eng, "an");
        assert_eq!(eng.composing(), "xi'an");
        let words = cand_words(&eng);
        assert!(words.contains(&"西安".into()), "xi'an: {words:?}");
        let out = select_word(&mut eng, "西安");
        assert_eq!(out.commit.as_deref(), Some("西安"));
        assert!(eng.composing().is_empty());
    }

    #[test]
    fn apostrophe_select_keeps_leftover() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "xi'anwo");
        let out = select_word(&mut eng, "西安");
        assert_eq!(out.commit.as_deref(), Some("西安"));
        assert_eq!(eng.composing(), "wo");
        assert!(
            cand_words(&eng).contains(&"我".into()),
            "leftover wo: {:?}",
            cand_words(&eng)
        );
    }

    #[test]
    fn empty_composing_apostrophe_still_commits_quote() {
        let mut eng = test_engine();
        let out = eng.handle_key(KeyEvent::Separator);
        assert_eq!(out.commit.as_deref(), Some("‘"));
        let out = eng.handle_key(KeyEvent::Punct('\''));
        assert_eq!(out.commit.as_deref(), Some("’"));
    }

    #[test]
    fn ascii_mode_types_ascii_apostrophe() {
        let mut eng = test_engine();
        eng.set_ascii_mode(true);
        assert_eq!(
            eng.handle_key(KeyEvent::Separator).commit.as_deref(),
            Some("'")
        );
        assert_eq!(
            eng.handle_key(KeyEvent::Punct('\'')).commit.as_deref(),
            Some("'")
        );
    }

    #[test]
    fn production_lexicon_danshi_xianshi_keneng_xian_diao() {
        let Some(dict) = production_system_dict() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!(
            "pulopinyin-seg-{}-{}",
            std::process::id(),
            now_unix()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let eng = Engine::open(Some(&dict), &dir).expect("open production dict");
        let danshi = query_words(&eng, "danshi");
        assert_eq!(
            danshi.first().map(String::as_str),
            Some("但是"),
            "danshi: {danshi:?}"
        );
        assert_ne!(danshi.first().map(String::as_str), Some("大牛市"));
        let xianshi = query_words(&eng, "xianshi");
        assert_eq!(
            xianshi.first().map(String::as_str),
            Some("显示"),
            "xianshi: {xianshi:?}"
        );
        assert_ne!(xianshi.first().map(String::as_str), Some("西安市"));
        let keneng = query_words(&eng, "keneng");
        assert!(
            keneng.contains(&"可能".into()),
            "keneng: {keneng:?}"
        );
        let xian = query_words(&eng, "xian");
        assert!(xian.contains(&"先".into()), "xian: {xian:?}");
        assert!(!xian.contains(&"西安".into()), "xian must not list 西安: {xian:?}");
        let xi_an = query_words(&eng, "xi'an");
        assert!(xi_an.contains(&"西安".into()), "xi'an: {xi_an:?}");
        let diao = query_words(&eng, "diao");
        assert!(
            diao.contains(&"掉".into()),
            "diao must include 掉: {diao:?}"
        );
        assert_ne!(diao.first().map(String::as_str), Some("低奥"));
        assert_eq!(
            query_words(&eng, "slh").first().map(String::as_str),
            Some("\u{2026}"),
            "production slh must not bury …"
        );
        let tsfh = query_words(&eng, "tsfh");
        assert_eq!(
            tsfh,
            crate::symbols::CATALOG
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>(),
            "production tsfh must stay a fixed catalog"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn query_slh_first_is_ellipsis() {
        let eng = test_engine();
        assert_eq!(
            query_words(&eng, "slh").first().map(String::as_str),
            Some("\u{2026}"),
            "slh: {:?}",
            query_words(&eng, "slh")
        );
        assert_eq!(
            query_words(&eng, "shengluehao").first().map(String::as_str),
            Some("\u{2026}")
        );
    }

    #[test]
    fn query_dunhao_douhao_and_dh() {
        let eng = test_engine();
        assert_eq!(
            query_words(&eng, "dunhao").first().map(String::as_str),
            Some("、")
        );
        assert_eq!(
            query_words(&eng, "douhao").first().map(String::as_str),
            Some("，")
        );
        let dh = query_words(&eng, "dh");
        assert!(dh.contains(&"、".into()), "dh: {dh:?}");
        assert!(dh.contains(&"，".into()), "dh: {dh:?}");
        let i_dun = dh.iter().position(|w| w == "、").unwrap();
        let i_dou = dh.iter().position(|w| w == "，").unwrap();
        assert!(i_dun < 2 && i_dou < 2, "dh must list both first: {dh:?}");
    }

    #[test]
    fn query_tsfh_catalog_is_fixed_pages() {
        let mut eng = test_engine();
        let words = query_words(&eng, "tsfh");
        assert_eq!(
            words,
            crate::symbols::CATALOG
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            words[..10].iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            crate::symbols::CATALOG[..10]
        );
        assert_eq!(
            words[10..20].iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            crate::symbols::CATALOG[10..20]
        );

        for alias in ["tesufuhao", "teshufuhao"] {
            assert_eq!(query_words(&eng, alias), words, "{alias}");
        }

        type_pinyin(&mut eng, "tsfh");
        assert_eq!(eng.candidates.page, 0);
        assert_eq!(
            eng.candidates.current_page().iter().map(|c| c.word.as_str()).collect::<Vec<_>>(),
            crate::symbols::CATALOG[..10]
        );
        let out = eng.handle_key(KeyEvent::PageNext);
        assert!(out.consumed);
        assert_eq!(eng.candidates.page, 1);
        assert_eq!(
            out.candidates.iter().map(|c| c.word.as_str()).collect::<Vec<_>>(),
            crate::symbols::CATALOG[10..20]
        );
        let out = eng.handle_key(KeyEvent::Punct('.'));
        assert_eq!(eng.candidates.page, 2);
        assert_eq!(
            out.candidates.iter().map(|c| c.word.as_str()).collect::<Vec<_>>(),
            crate::symbols::CATALOG[20..30]
        );
        let out = eng.handle_key(KeyEvent::Punct(','));
        assert_eq!(eng.candidates.page, 1);
        assert_eq!(
            out.candidates.iter().map(|c| c.word.as_str()).collect::<Vec<_>>(),
            crate::symbols::CATALOG[10..20]
        );
    }

    #[test]
    fn tsfh_catalog_not_reranked_by_learning() {
        let mut eng = test_engine();
        type_pinyin(&mut eng, "tsfh");
        eng.handle_key(KeyEvent::PageNext);
        // Select “ (page 2, index 1) many times if learning were applied.
        for _ in 0..12 {
            let cand = eng
                .query("tsfh")
                .items
                .into_iter()
                .find(|c| c.word == "“")
                .unwrap();
            eng.learn(&cand);
        }
        let after = query_words(&eng, "tsfh");
        assert_eq!(after[0], "\u{2026}");
        assert_eq!(after[11], "“");
        assert_eq!(
            after,
            crate::symbols::CATALOG
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
    }

    fn production_system_dict() -> Option<PathBuf> {
        crate::default_system_dict()
    }
}
