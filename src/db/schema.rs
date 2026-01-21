//! Database schema initialization and migrations

use anyhow::Result;
use rusqlite::Connection;

/// Initialize the database schema
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tools (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            category TEXT,
            source TEXT NOT NULL DEFAULT 'unknown',
            install_command TEXT,
            binary_name TEXT,
            is_installed INTEGER NOT NULL DEFAULT 0,
            is_favorite INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS interests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            priority INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_path TEXT NOT NULL,
            target_path TEXT NOT NULL,
            tool_id INTEGER REFERENCES tools(id),
            is_symlinked INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tools_name ON tools(name);
        CREATE INDEX IF NOT EXISTS idx_tools_category ON tools(category);
        CREATE INDEX IF NOT EXISTS idx_tools_source ON tools(source);
        CREATE INDEX IF NOT EXISTS idx_tools_installed ON tools(is_installed);

        CREATE TABLE IF NOT EXISTS bundles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS bundle_tools (
            bundle_id INTEGER NOT NULL REFERENCES bundles(id) ON DELETE CASCADE,
            tool_name TEXT NOT NULL,
            PRIMARY KEY (bundle_id, tool_name)
        );

        CREATE TABLE IF NOT EXISTS tool_labels (
            tool_id INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
            label TEXT NOT NULL,
            PRIMARY KEY (tool_id, label)
        );

        CREATE TABLE IF NOT EXISTS tool_github (
            tool_id INTEGER PRIMARY KEY REFERENCES tools(id) ON DELETE CASCADE,
            repo_owner TEXT NOT NULL,
            repo_name TEXT NOT NULL,
            description TEXT,
            stars INTEGER DEFAULT 0,
            language TEXT,
            homepage TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tool_usage (
            tool_id INTEGER PRIMARY KEY REFERENCES tools(id) ON DELETE CASCADE,
            use_count INTEGER NOT NULL DEFAULT 0,
            last_used TEXT,
            first_seen TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Daily usage tracking for sparklines
        CREATE TABLE IF NOT EXISTS usage_daily (
            tool_id INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
            date TEXT NOT NULL,  -- YYYY-MM-DD format
            count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (tool_id, date)
        );

        CREATE INDEX IF NOT EXISTS idx_usage_daily_date ON usage_daily(date);

        CREATE TABLE IF NOT EXISTS extraction_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_owner TEXT NOT NULL,
            repo_name TEXT NOT NULL,
            version TEXT NOT NULL,
            name TEXT NOT NULL,
            binary TEXT,
            source TEXT NOT NULL,
            install_command TEXT,
            description TEXT NOT NULL,
            category TEXT NOT NULL,
            extracted_at TEXT NOT NULL,
            UNIQUE(repo_owner, repo_name)
        );

        CREATE INDEX IF NOT EXISTS idx_bundles_name ON bundles(name);
        CREATE INDEX IF NOT EXISTS idx_tool_labels_label ON tool_labels(label);
        CREATE INDEX IF NOT EXISTS idx_extraction_cache_repo ON extraction_cache(repo_owner, repo_name);

        CREATE TABLE IF NOT EXISTS ai_cache (
            cache_key TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS discover_search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL,
            ai_enabled INTEGER NOT NULL DEFAULT 0,
            source_filters TEXT NOT NULL,  -- JSON array of enabled sources
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_discover_search_history_created ON discover_search_history(created_at DESC);
        "#,
    )?;

    // Run migrations for new columns
    run_migrations(conn)?;

    Ok(())
}

/// Check if a column exists in a table
fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
    let query = format!("PRAGMA table_info({})", table);
    conn.prepare(&query)
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(1))
                .map(|rows| rows.filter_map(|r| r.ok()).any(|name| name == column))
        })
        .unwrap_or(false)
}

/// Run database migrations for schema updates
fn run_migrations(conn: &Connection) -> Result<()> {
    // Migration: Add version tracking columns to tools table
    if !column_exists(conn, "tools", "installed_version") {
        conn.execute("ALTER TABLE tools ADD COLUMN installed_version TEXT", [])?;
    }

    if !column_exists(conn, "tools", "available_version") {
        conn.execute("ALTER TABLE tools ADD COLUMN available_version TEXT", [])?;
    }

    if !column_exists(conn, "tools", "version_policy") {
        conn.execute("ALTER TABLE tools ADD COLUMN version_policy TEXT", [])?;
    }

    // Migration: Add version_policy column to bundles table
    if !column_exists(conn, "bundles", "version_policy") {
        conn.execute("ALTER TABLE bundles ADD COLUMN version_policy TEXT", [])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"tools".to_string()));
        assert!(tables.contains(&"bundles".to_string()));
    }

    #[test]
    fn test_migrations_add_version_columns() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify version columns exist in tools
        assert!(column_exists(&conn, "tools", "installed_version"));
        assert!(column_exists(&conn, "tools", "available_version"));
        assert!(column_exists(&conn, "tools", "version_policy"));

        // Verify version_policy exists in bundles
        assert!(column_exists(&conn, "bundles", "version_policy"));
    }

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Running migrations again should not fail
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }
}
