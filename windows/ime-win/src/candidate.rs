//! Candidate UI model (2×5, same as macOS / Linux).
//!
//! The Win32 HWND is not created yet. TSF will own composition; this module
//! only lays out the current engine page so a later `CreateWindowExW` can draw it.

use engine::{Candidate, PAGE_SIZE};

pub const COLS: usize = 5;
pub const ROWS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub label: String,
}

/// Number-select labels for the current page (`1.`…`9.` `0.`).
pub fn layout_cells(cands: &[Candidate]) -> Vec<Cell> {
    cands
        .iter()
        .take(PAGE_SIZE)
        .enumerate()
        .map(|(i, c)| {
            let n = if i == 9 { 0 } else { i + 1 };
            Cell {
                row: i / COLS,
                col: i % COLS,
                label: format!("{n}.{word}", word = c.word),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::Candidate;

    #[test]
    fn ten_cells_fill_two_rows() {
        let cands: Vec<Candidate> = (0..10)
            .map(|i| Candidate {
                word: format!("w{i}"),
                syllables: vec!["a".into()],
                base_frequency: 0,
                user_count: 0,
                last_used: None,
                score: 0.0,
                consumed: 1,
                complete: true,
            })
            .collect();
        let cells = layout_cells(&cands);
        assert_eq!(cells.len(), 10);
        assert_eq!(cells[0].row, 0);
        assert_eq!(cells[0].col, 0);
        assert_eq!(cells[5].row, 1);
        assert_eq!(cells[9].label, "0.w9");
    }
}
