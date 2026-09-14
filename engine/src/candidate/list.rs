use super::pagination::{page_count, page_slice};

#[derive(Debug, Clone)]
pub struct Candidate {
    pub word: String,
    pub syllables: Vec<String>,
    pub base_frequency: u32,
    pub user_count: u32,
    pub last_used: Option<u64>,
    pub score: f64,
    /// Bytes of the current composing string this candidate consumes.
    pub consumed: usize,
    /// Word pinyin was fully matched (not an unfinished longer word).
    pub complete: bool,
}

impl Candidate {
    pub fn pinyin_spaced(&self) -> String {
        self.syllables.join(" ")
    }
}

#[derive(Debug, Clone, Default)]
pub struct CandidateList {
    pub items: Vec<Candidate>,
    pub page: usize,
}

impl CandidateList {
    pub fn from_sorted(items: Vec<Candidate>) -> Self {
        Self { items, page: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn page_count(&self) -> usize {
        page_count(self.items.len())
    }

    pub fn current_page(&self) -> &[Candidate] {
        page_slice(&self.items, self.page)
    }

    pub fn next_page(&mut self) {
        let n = self.page_count();
        if n == 0 {
            self.page = 0;
            return;
        }
        self.page = (self.page + 1).min(n - 1);
    }

    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    /// Digit 1–9 selects index 0–8, 0 selects index 9 on the current page.
    pub fn select_digit(&self, digit: u8) -> Option<&Candidate> {
        let page = self.current_page();
        let idx = match digit {
            0 => 9,
            1..=9 => (digit as usize) - 1,
            _ => return None,
        };
        page.get(idx)
    }

    pub fn first(&self) -> Option<&Candidate> {
        self.items.first()
    }
}
