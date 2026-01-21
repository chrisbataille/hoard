//! Bundle database operations

use anyhow::Result;
use rusqlite::params;

use crate::models::{Bundle, VersionPolicy};

use super::Database;
use super::tools::parse_datetime;

impl Database {
    // ==================== Bundle Operations ====================

    /// Create a new bundle
    pub fn create_bundle(&self, bundle: &Bundle) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO bundles (name, description, created_at, version_policy) VALUES (?1, ?2, ?3, ?4)",
            params![
                bundle.name,
                bundle.description,
                bundle.created_at.to_rfc3339(),
                bundle.version_policy.as_ref().map(|p| p.to_string()),
            ],
        )?;

        let bundle_id = self.conn.last_insert_rowid();

        // Insert bundle tools in transaction
        for tool_name in &bundle.tools {
            tx.execute(
                "INSERT INTO bundle_tools (bundle_id, tool_name) VALUES (?1, ?2)",
                params![bundle_id, tool_name],
            )?;
        }

        tx.commit()?;
        Ok(bundle_id)
    }

    /// Get a bundle by name
    pub fn get_bundle(&self, name: &str) -> Result<Option<Bundle>> {
        let bundle_row = self.conn.query_row(
            "SELECT id, name, description, created_at, version_policy FROM bundles WHERE name = ?1",
            [name],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        );

        match bundle_row {
            Ok((id, name, description, created_at, version_policy)) => {
                // Get tools for this bundle
                let mut stmt = self.conn.prepare(
                    "SELECT tool_name FROM bundle_tools WHERE bundle_id = ?1 ORDER BY tool_name",
                )?;
                let tools: Vec<String> =
                    stmt.query_map([id], |row| row.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;

                Ok(Some(Bundle {
                    id: Some(id),
                    name,
                    description,
                    tools,
                    version_policy: version_policy.map(|s| VersionPolicy::from(s.as_str())),
                    created_at: parse_datetime(created_at),
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all bundles
    pub fn list_bundles(&self) -> Result<Vec<Bundle>> {
        // Single query with LEFT JOIN to get bundles and their tools
        let mut stmt = self.conn.prepare(
            "SELECT b.id, b.name, b.description, b.created_at, b.version_policy, bt.tool_name
             FROM bundles b
             LEFT JOIN bundle_tools bt ON b.id = bt.bundle_id
             ORDER BY b.name, bt.tool_name",
        )?;

        // Group rows by bundle
        let mut bundles: Vec<Bundle> = Vec::new();
        let mut current_id: Option<i64> = None;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let created_at: String = row.get(3)?;
            let version_policy: Option<String> = row.get(4)?;
            let tool_name: Option<String> = row.get(5)?;
            if current_id != Some(id) {
                // New bundle
                bundles.push(Bundle {
                    id: Some(id),
                    name,
                    description,
                    tools: tool_name.into_iter().collect(),
                    version_policy: version_policy.map(|s| VersionPolicy::from(s.as_str())),
                    created_at: parse_datetime(created_at),
                });
                current_id = Some(id);
            } else if let Some(tool) = tool_name {
                // Add tool to current bundle
                if let Some(bundle) = bundles.last_mut() {
                    bundle.tools.push(tool);
                }
            }
        }

        Ok(bundles)
    }

    /// Delete a bundle by name
    pub fn delete_bundle(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM bundles WHERE name = ?1", [name])?;
        Ok(rows > 0)
    }

    /// Add tools to an existing bundle
    pub fn add_to_bundle(&self, bundle_name: &str, tools: &[String]) -> Result<bool> {
        let bundle_id: i64 = match self.conn.query_row(
            "SELECT id FROM bundles WHERE name = ?1",
            [bundle_name],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        let tx = self.conn.unchecked_transaction()?;
        for tool_name in tools {
            // Use INSERT OR IGNORE to skip duplicates
            tx.execute(
                "INSERT OR IGNORE INTO bundle_tools (bundle_id, tool_name) VALUES (?1, ?2)",
                params![bundle_id, tool_name],
            )?;
        }
        tx.commit()?;

        Ok(true)
    }

    /// Remove tools from a bundle
    pub fn remove_from_bundle(&self, bundle_name: &str, tools: &[String]) -> Result<bool> {
        let bundle_id: i64 = match self.conn.query_row(
            "SELECT id FROM bundles WHERE name = ?1",
            [bundle_name],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        let tx = self.conn.unchecked_transaction()?;
        for tool_name in tools {
            tx.execute(
                "DELETE FROM bundle_tools WHERE bundle_id = ?1 AND tool_name = ?2",
                params![bundle_id, tool_name],
            )?;
        }
        tx.commit()?;

        Ok(true)
    }

    /// Get all bundle names (for completions)
    pub fn get_bundle_names(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM bundles ORDER BY name")?;
        let names = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// Set version policy for a bundle
    pub fn set_bundle_version_policy(
        &self,
        name: &str,
        policy: Option<&VersionPolicy>,
    ) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE bundles SET version_policy = ?1 WHERE name = ?2",
            params![policy.map(|p| p.to_string()), name],
        )?;
        Ok(rows > 0)
    }
}
