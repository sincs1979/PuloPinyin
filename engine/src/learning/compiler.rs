use super::LearnedDb;
use crate::dictionary::{BinaryDict, DictEntry};
use std::collections::HashMap;
use std::path::Path;

pub const COMPILE_UNCOMPILED_THRESHOLD: u32 = 1000;
pub const COMPILE_INTERVAL_SECS: u64 = 7 * 24 * 3600;

pub fn should_compile(db: &LearnedDb, now: u64) -> crate::Result<bool> {
    if db.uncompiled_count()? >= COMPILE_UNCOMPILED_THRESHOLD {
        return Ok(true);
    }
    match db.last_compile_unix()? {
        None => Ok(db.uncompiled_count()? > 0),
        Some(ts) => {
            Ok(now.saturating_sub(ts) >= COMPILE_INTERVAL_SECS && db.uncompiled_count()? > 0)
        }
    }
}

/// Rebuild `user.dict` from the full learned.db (source of truth).
pub fn compile_user_dict(
    db: &LearnedDb,
    dest: impl AsRef<Path>,
    now: u64,
) -> crate::Result<BinaryDict> {
    let records = db.load_all()?;
    let mut merged: HashMap<(String, String), (u32, u64)> = HashMap::new();
    for rec in records {
        let key = (rec.word, rec.pinyin);
        let slot = merged.entry(key).or_insert((0, 0));
        slot.0 = slot.0.saturating_add(rec.count);
        slot.1 = slot.1.max(rec.last_used);
    }
    let mut entries: Vec<DictEntry> = merged
        .into_iter()
        .map(|((word, pinyin), (count, _))| {
            let syllables = crate::dictionary::binary::learned_syllables(&word, &pinyin);
            DictEntry {
                word,
                syllables,
                frequency: count,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        b.frequency
            .cmp(&a.frequency)
            .then_with(|| a.word.cmp(&b.word))
    });
    let dict = BinaryDict::from_entries(entries);
    dict.write_to(dest)?;
    db.mark_compiled_and_touch(now)?;
    Ok(dict)
}
