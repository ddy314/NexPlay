use rusqlite::{Connection, Transaction};
use rusqlite_migration::{M, Migrations};

use crate::error::AppResult;

const BASELINE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS media_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    file_hash TEXT,
    match_ignored INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_media_items_deleted_at
    ON media_items(deleted_at);

CREATE TABLE IF NOT EXISTS watch_progress (
    media_id INTEGER PRIMARY KEY,
    position_ms INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(media_id) REFERENCES media_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS subjects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    provider_subject_id TEXT NOT NULL,
    title TEXT NOT NULL,
    title_cn TEXT,
    summary TEXT,
    air_date TEXT,
    rating REAL,
    rank INTEGER,
    image_large TEXT,
    image_common TEXT,
    tags TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider, provider_subject_id)
);

CREATE TABLE IF NOT EXISTS media_subject_links (
    media_id INTEGER NOT NULL,
    subject_id INTEGER NOT NULL,
    match_source TEXT NOT NULL,
    confidence REAL,
    confirmed INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(media_id, subject_id),
    FOREIGN KEY(media_id) REFERENCES media_items(id) ON DELETE CASCADE,
    FOREIGN KEY(subject_id) REFERENCES subjects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_media_subject_links_media
    ON media_subject_links(media_id);

CREATE TABLE IF NOT EXISTS subject_image_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id INTEGER NOT NULL,
    image_kind TEXT NOT NULL,
    source_url TEXT NOT NULL,
    local_path TEXT NOT NULL,
    content_hash TEXT,
    width INTEGER,
    height INTEGER,
    downloaded_at INTEGER NOT NULL,
    last_accessed_at INTEGER NOT NULL,
    UNIQUE(subject_id, image_kind),
    FOREIGN KEY(subject_id) REFERENCES subjects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS episodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subject_id INTEGER NOT NULL,
    provider_episode_id TEXT,
    sort_number REAL,
    title TEXT,
    title_cn TEXT,
    air_date TEXT,
    FOREIGN KEY(subject_id) REFERENCES subjects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS media_episode_links (
    media_id INTEGER NOT NULL,
    episode_id INTEGER,
    episode_title TEXT,
    episode_number REAL,
    match_source TEXT,
    confidence REAL,
    FOREIGN KEY(media_id) REFERENCES media_items(id) ON DELETE CASCADE,
    FOREIGN KEY(episode_id) REFERENCES episodes(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS metadata_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_type TEXT NOT NULL,
    target_id INTEGER NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS metadata_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER NOT NULL,
    provider TEXT NOT NULL,
    provider_subject_id TEXT NOT NULL,
    title TEXT NOT NULL,
    title_cn TEXT,
    summary TEXT,
    air_date TEXT,
    rating REAL,
    rank INTEGER,
    image_large TEXT,
    image_common TEXT,
    confidence REAL,
    source TEXT NOT NULL,
    selected INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(media_id, provider, provider_subject_id),
    FOREIGN KEY(media_id) REFERENCES media_items(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_episodes_subject_provider
    ON episodes(subject_id, provider_episode_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_media_episode_links_media
    ON media_episode_links(media_id);

CREATE TABLE IF NOT EXISTS danmaku_matches (
    media_id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL,
    title TEXT NOT NULL,
    anime_id INTEGER,
    episode_id INTEGER,
    anime_title TEXT,
    episode TEXT,
    comment_count INTEGER NOT NULL DEFAULT 0,
    exact INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(media_id) REFERENCES media_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS external_subject_mappings (
    provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    subject_id INTEGER NOT NULL,
    title TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(provider, external_id),
    FOREIGN KEY(subject_id) REFERENCES subjects(id) ON DELETE CASCADE
);
"#;

pub fn init_database(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let migrations = Migrations::new(vec![
        M::up_with_hook(BASELINE_SCHEMA, |tx: &Transaction| {
            add_column_if_missing(
                tx,
                "media_items",
                "match_ignored",
                "INTEGER NOT NULL DEFAULT 0",
            )?;
            Ok(())
        })
        .comment("baseline NexPlay media library schema"),
    ]);
    migrations.to_latest(conn)?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let existing: String = row.get(1)?;
        if existing == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_fresh_database_at_baseline_version() {
        let mut conn = Connection::open_in_memory().expect("open db");
        init_database(&mut conn).expect("migrate");

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read version");
        assert_eq!(version, 1);
        assert!(column_exists(&conn, "media_items", "match_ignored"));
    }

    #[test]
    fn upgrades_legacy_database_missing_match_ignored() {
        let mut conn = Connection::open_in_memory().expect("open db");
        conn.execute_batch(
            r#"
            CREATE TABLE media_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                file_hash TEXT,
                deleted_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )
        .expect("legacy schema");

        init_database(&mut conn).expect("migrate");

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read version");
        assert_eq!(version, 1);
        assert!(column_exists(&conn, "media_items", "match_ignored"));
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("table info");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("columns");
        rows.filter_map(Result::ok).any(|name| name == column)
    }
}
