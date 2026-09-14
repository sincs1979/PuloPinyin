//! Numeric [`score`] is habit + public frequency + recency only.
//!
//! Query sort applies structural keys *before* that score: complete match,
//! then bytes consumed. Valid full-syllable cuts of the same input
//! (`keneng` → ke+neng and ken+eng) compete by score, not by a fixed cut.
//! Atomic syllables (`xian`, `xianshi`) and no-initial-skip (`danshi`)
//! are enforced in matching, not by preferring the greedy parse here.

pub mod score;

pub use score::{score, Recency, ScoreInputs};
