use tracing::info;

use engram_core::EngramError;

/// A single migration: version + SQL embedded via include_str!.
pub struct Migration {
    pub version: i32,
    pub sql: &'static str,
}

/// All migrations in order. Add new ones here.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("migrations/002_fts.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("migrations/003_vectors.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("migrations/004_graph.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("migrations/006_capsules.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("migrations/007_cross_project.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("migrations/008_episodic_semantic.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("migrations/009_review_schedule.sql"),
    },
    Migration {
        version: 11,
        sql: include_str!("migrations/011_attachments.sql"),
    },
    Migration {
        version: 12,
        sql: include_str!("migrations/012_boundaries.sql"),
    },
    Migration {
        version: 13,
        sql: include_str!("migrations/013_agent_personalities.sql"),
    },
    Migration {
        version: 15,
        sql: include_str!("migrations/015_beliefs.sql"),
    },
    Migration {
        version: 16,
        sql: include_str!("migrations/016_entities.sql"),
    },
];

/// Run all pending migrations on a connection.
pub fn run_migrations(conn: &rusqlite::Connection) -> crate::Result<()> {
    // Create tracking table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| EngramError::Database(e.to_string()))?;

    for migration in MIGRATIONS {
        // Check if already applied
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE version = ?",
                [migration.version],
                |row| row.get(0),
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        if exists {
            continue;
        }

        info!("Applying migration {:03}", migration.version);

        conn.execute_batch(migration.sql)
            .map_err(|e| EngramError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO _migrations (version) VALUES (?)",
            [migration.version],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        info!("Migration {:03} applied", migration.version);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_idempotent() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap(); // second run is no-op

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i32);
    }

    #[test]
    fn first_run_applies_all() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Check tables exist
        let tables = [
            "sessions",
            "observations",
            "prompts",
            "_migrations",
            "observations_fts",
        ];
        for table in &tables {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE name = ?",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "table {table} should exist after migration");
        }
    }
}
