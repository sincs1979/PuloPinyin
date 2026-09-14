use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct LearnRecord {
    pub word: String,
    pub pinyin: String,
    pub count: u32,
    pub last_used: u64,
    pub compiled: bool,
}

/// SQLite is the durable source of truth for user learning.
///
/// The in-memory overlay in `Engine` is what queries read.
/// This type is the persistence layer (and the rebuild source for user.dict).
pub struct LearnedDb {
    path: PathBuf,
    conn: Mutex<Connection>,
}

impl LearnedDb {
    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn open(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS learned (
                word TEXT NOT NULL,
                pinyin TEXT NOT NULL,
                user_count INTEGER NOT NULL DEFAULT 0,
                last_used INTEGER NOT NULL DEFAULT 0,
                compiled INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (word, pinyin)
            );
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(Self {
            path,
            conn: Mutex::new(conn),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bump(&self, word: &str, pinyin: &str, now: u64) -> crate::Result<u32> {
        let conn = self.lock();
        conn.execute(
            "
            INSERT INTO learned (word, pinyin, user_count, last_used, compiled)
            VALUES (?1, ?2, 1, ?3, 0)
            ON CONFLICT(word, pinyin) DO UPDATE SET
                user_count = user_count + 1,
                last_used = excluded.last_used,
                compiled = 0
            ",
            params![word, pinyin, now as i64],
        )?;
        let count: u32 = conn.query_row(
            "SELECT user_count FROM learned WHERE word = ?1 AND pinyin = ?2",
            params![word, pinyin],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn get(&self, word: &str, pinyin: &str) -> crate::Result<Option<LearnRecord>> {
        let conn = self.lock();
        let rec = conn
            .query_row(
                "SELECT word, pinyin, user_count, last_used, compiled FROM learned WHERE word = ?1 AND pinyin = ?2",
                params![word, pinyin],
                row_to_record,
            )
            .optional()?;
        Ok(rec)
    }

    pub fn load_all(&self) -> crate::Result<Vec<LearnRecord>> {
        let conn = self.lock();
        let mut stmt =
            conn.prepare("SELECT word, pinyin, user_count, last_used, compiled FROM learned")?;
        let rows = stmt.query_map([], row_to_record)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn uncompiled_count(&self) -> crate::Result<u32> {
        let conn = self.lock();
        let n: u32 = conn.query_row(
            "SELECT COUNT(*) FROM learned WHERE compiled = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(n)
    }

    pub fn last_compile_unix(&self) -> crate::Result<Option<u64>> {
        let conn = self.lock();
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'last_compile'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.and_then(|s| s.parse().ok()))
    }

    pub fn mark_compiled_and_touch(&self, now: u64) -> crate::Result<()> {
        let conn = self.lock();
        conn.execute("UPDATE learned SET compiled = 1", [])?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('last_compile', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![now.to_string()],
        )?;
        Ok(())
    }
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<LearnRecord> {
    let compiled: i64 = row.get(4)?;
    Ok(LearnRecord {
        word: row.get(0)?,
        pinyin: row.get(1)?,
        count: row.get::<_, u32>(2)?,
        last_used: row.get::<_, i64>(3)? as u64,
        compiled: compiled != 0,
    })
}
