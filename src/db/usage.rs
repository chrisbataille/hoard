//! Usage tracking database operations

use anyhow::Result;
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::models::Tool;

use super::Database;
use super::tools::tool_from_row;

/// Tool usage statistics
#[derive(Debug, Clone)]
pub struct ToolUsage {
    pub use_count: i64,
    pub last_used: Option<String>,
    pub first_seen: String,
}

impl Database {
    // ==================== Usage Tracking ====================

    /// Record tool usage (increment count or insert new record)
    pub fn record_usage(
        &self,
        tool_name: &str,
        count: i64,
        last_used: Option<&str>,
    ) -> Result<bool> {
        let tool_id: i64 =
            match self
                .conn
                .query_row("SELECT id FROM tools WHERE name = ?1", [tool_name], |row| {
                    row.get(0)
                }) {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
                Err(e) => return Err(e.into()),
            };

        let now = Utc::now().to_rfc3339();

        // Try to update existing record, or insert new one
        let updated = self.conn.execute(
            "UPDATE tool_usage SET use_count = use_count + ?1, last_used = COALESCE(?2, last_used), updated_at = ?3 WHERE tool_id = ?4",
            params![count, last_used, now, tool_id],
        )?;

        if updated == 0 {
            self.conn.execute(
                "INSERT INTO tool_usage (tool_id, use_count, last_used, first_seen, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![tool_id, count, last_used, now, now],
            )?;
        }

        Ok(true)
    }

    /// Match a command to a tracked tool by binary or name
    /// Returns the tool name if found, None otherwise
    pub fn match_command_to_tool(&self, cmd: &str) -> Result<Option<String>> {
        // First try to match by binary name, then by tool name
        let result = self.conn.query_row(
            "SELECT name FROM tools WHERE binary_name = ?1 OR name = ?1 LIMIT 1",
            [cmd],
            |row| row.get(0),
        );

        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get usage stats for a tool
    pub fn get_usage(&self, tool_name: &str) -> Result<Option<ToolUsage>> {
        let mut stmt = self.conn.prepare(
            "SELECT tu.use_count, tu.last_used, tu.first_seen
             FROM tool_usage tu
             INNER JOIN tools t ON tu.tool_id = t.id
             WHERE t.name = ?1",
        )?;

        let usage = stmt
            .query_row([tool_name], |row| {
                Ok(ToolUsage {
                    use_count: row.get(0)?,
                    last_used: row.get(1)?,
                    first_seen: row.get(2)?,
                })
            })
            .optional()?;

        Ok(usage)
    }

    /// Get all usage stats sorted by count (most used first)
    pub fn get_all_usage(&self) -> Result<Vec<(String, ToolUsage)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, tu.use_count, tu.last_used, tu.first_seen
             FROM tool_usage tu
             INNER JOIN tools t ON tu.tool_id = t.id
             ORDER BY tu.use_count DESC",
        )?;

        let results = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    ToolUsage {
                        use_count: row.get(1)?,
                        last_used: row.get(2)?,
                        first_seen: row.get(3)?,
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get list of tool names and their binary names for matching against history
    pub fn get_tool_binaries(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, COALESCE(binary_name, name) as binary FROM tools")?;

        let results = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Clear all usage data
    pub fn clear_usage(&self) -> Result<()> {
        self.conn.execute("DELETE FROM tool_usage", [])?;
        Ok(())
    }

    /// Count orphaned usage records (tool_id doesn't exist in tools)
    pub fn count_orphaned_usage(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_usage WHERE tool_id NOT IN (SELECT id FROM tools)",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Delete orphaned usage records
    pub fn delete_orphaned_usage(&self) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM tool_usage WHERE tool_id NOT IN (SELECT id FROM tools)",
            [],
        )?;
        Ok(deleted)
    }

    /// Get installed tools with no usage data (never used)
    pub fn get_unused_tools(&self) -> Result<Vec<Tool>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.description, t.category, t.source, t.install_command,
                    t.binary_name, t.is_installed, t.is_favorite, t.notes, t.created_at, t.updated_at
             FROM tools t
             LEFT JOIN tool_usage tu ON t.id = tu.tool_id
             WHERE t.is_installed = 1 AND (tu.tool_id IS NULL OR tu.use_count = 0)
             ORDER BY t.name",
        )?;

        let tools = stmt
            .query_map([], tool_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tools)
    }
}
