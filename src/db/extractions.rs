//! Extraction cache and AI cache database operations

use anyhow::Result;
use rusqlite::params;

use super::Database;

/// Cached extraction from a GitHub README
#[derive(Debug, Clone)]
pub struct CachedExtraction {
    pub repo_owner: String,
    pub repo_name: String,
    pub version: String,
    pub name: String,
    pub binary: Option<String>,
    pub source: String,
    pub install_command: Option<String>,
    pub description: String,
    pub category: String,
    pub extracted_at: String,
}

impl Database {
    // ==================== Extraction Cache ====================

    /// Get cached extraction for a repository if version matches
    pub fn get_cached_extraction(
        &self,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Option<CachedExtraction>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT repo_owner, repo_name, version, name, binary, source,
                   install_command, description, category, extracted_at
            FROM extraction_cache
            WHERE repo_owner = ?1 AND repo_name = ?2 AND version = ?3
            "#,
        )?;

        let mut rows = stmt.query(params![owner, repo, version])?;

        if let Some(row) = rows.next()? {
            Ok(Some(CachedExtraction {
                repo_owner: row.get(0)?,
                repo_name: row.get(1)?,
                version: row.get(2)?,
                name: row.get(3)?,
                binary: row.get(4)?,
                source: row.get(5)?,
                install_command: row.get(6)?,
                description: row.get(7)?,
                category: row.get(8)?,
                extracted_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Cache an extraction (upserts if repo already exists)
    pub fn cache_extraction(&self, extraction: &CachedExtraction) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO extraction_cache
                (repo_owner, repo_name, version, name, binary, source,
                 install_command, description, category, extracted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(repo_owner, repo_name) DO UPDATE SET
                version = excluded.version,
                name = excluded.name,
                binary = excluded.binary,
                source = excluded.source,
                install_command = excluded.install_command,
                description = excluded.description,
                category = excluded.category,
                extracted_at = excluded.extracted_at
            "#,
            params![
                extraction.repo_owner,
                extraction.repo_name,
                extraction.version,
                extraction.name,
                extraction.binary,
                extraction.source,
                extraction.install_command,
                extraction.description,
                extraction.category,
                extraction.extracted_at,
            ],
        )?;
        Ok(())
    }

    /// List all cached extractions
    pub fn list_cached_extractions(&self) -> Result<Vec<CachedExtraction>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT repo_owner, repo_name, version, name, binary, source,
                   install_command, description, category, extracted_at
            FROM extraction_cache
            ORDER BY extracted_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CachedExtraction {
                repo_owner: row.get(0)?,
                repo_name: row.get(1)?,
                version: row.get(2)?,
                name: row.get(3)?,
                binary: row.get(4)?,
                source: row.get(5)?,
                install_command: row.get(6)?,
                description: row.get(7)?,
                category: row.get(8)?,
                extracted_at: row.get(9)?,
            })
        })?;

        let mut extractions = Vec::new();
        for row in rows {
            extractions.push(row?);
        }
        Ok(extractions)
    }

    /// Clear extraction cache
    pub fn clear_extraction_cache(&self) -> Result<usize> {
        let count = self.conn.execute("DELETE FROM extraction_cache", [])?;
        Ok(count)
    }

    // ==================== AI Cache Operations ====================

    /// Get a cached value by key
    pub fn get_ai_cache(&self, key: &str) -> Result<Option<String>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT content FROM ai_cache WHERE cache_key = ?",
                [key],
                |row| row.get(0),
            )
            .ok();
        Ok(result)
    }

    /// Set a cached value
    pub fn set_ai_cache(&self, key: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO ai_cache (cache_key, content, created_at)
             VALUES (?, ?, datetime('now'))",
            rusqlite::params![key, content],
        )?;
        Ok(())
    }

    /// Delete a cached value
    pub fn delete_ai_cache(&self, key: &str) -> Result<bool> {
        let count = self
            .conn
            .execute("DELETE FROM ai_cache WHERE cache_key = ?", [key])?;
        Ok(count > 0)
    }
}
