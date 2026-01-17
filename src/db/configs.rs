//! Config database operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::models::Config;

use super::Database;
use super::tools::parse_datetime;

impl Database {
    // ==================== Config Operations ====================

    /// Insert a new config
    pub fn insert_config(&self, config: &Config) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO configs (name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                config.name,
                config.source_path,
                config.target_path,
                config.tool_id,
                config.is_symlinked,
                config.created_at.to_rfc3339(),
                config.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// List all configs
    pub fn list_configs(&self) -> Result<Vec<Config>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at
             FROM configs ORDER BY name",
        )?;

        let configs = stmt
            .query_map([], |row| {
                Ok(Config {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    target_path: row.get(3)?,
                    tool_id: row.get(4)?,
                    is_symlinked: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(configs)
    }

    /// Get a config by name
    pub fn get_config_by_name(&self, name: &str) -> Result<Option<Config>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at
             FROM configs WHERE name = ?1",
        )?;

        let config = stmt
            .query_row([name], |row| {
                Ok(Config {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    target_path: row.get(3)?,
                    tool_id: row.get(4)?,
                    is_symlinked: row.get(5)?,
                    created_at: parse_datetime(row.get(6)?),
                    updated_at: parse_datetime(row.get(7)?),
                })
            })
            .optional()?;

        Ok(config)
    }

    /// Get configs associated with a tool
    pub fn get_configs_for_tool(&self, tool_id: i64) -> Result<Vec<Config>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at
             FROM configs WHERE tool_id = ?1 ORDER BY name",
        )?;

        let configs = stmt
            .query_map([tool_id], |row| {
                Ok(Config {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    target_path: row.get(3)?,
                    tool_id: row.get(4)?,
                    is_symlinked: row.get(5)?,
                    created_at: parse_datetime(row.get(6)?),
                    updated_at: parse_datetime(row.get(7)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(configs)
    }

    /// Update a config's symlink status
    pub fn set_config_symlinked(&self, name: &str, is_symlinked: bool) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE configs SET is_symlinked = ?1, updated_at = ?2 WHERE name = ?3",
            params![is_symlinked, now, name],
        )?;
        Ok(())
    }

    /// Update a config's paths
    pub fn update_config_paths(
        &self,
        name: &str,
        source_path: &str,
        target_path: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE configs SET source_path = ?1, target_path = ?2, updated_at = ?3 WHERE name = ?4",
            params![source_path, target_path, now, name],
        )?;
        Ok(())
    }

    /// Link a config to a tool
    pub fn link_config_to_tool(&self, config_name: &str, tool_name: &str) -> Result<()> {
        let tool = self
            .get_tool_by_name(tool_name)?
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name))?;

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE configs SET tool_id = ?1, updated_at = ?2 WHERE name = ?3",
            params![tool.id, now, config_name],
        )?;
        Ok(())
    }

    /// Delete a config
    pub fn delete_config(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM configs WHERE name = ?1", [name])?;
        Ok(rows > 0)
    }
}

// Import OptionalExtension for .optional() method
use rusqlite::OptionalExtension;
