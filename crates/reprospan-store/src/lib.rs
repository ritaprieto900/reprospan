use std::path::Path;

use reprospan_core::Bundle;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use thiserror::Error;

pub struct Store {
    connection: Connection,
}

/// Lightweight bundle metadata for list endpoints.
#[derive(Debug, Serialize)]
pub struct BundleListItem {
    pub bundle_id: String,
    pub schema_version: String,
    pub created_at: String,
    pub imported_at: String,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("bundle already exists: {0}")]
    Conflict(String),
    #[error("bundle not found: {0}")]
    NotFound(String),
    #[error("invalid bundle: {0}")]
    InvalidBundle(#[from] reprospan_core::CoreError),
    #[error("database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("stored canonical document is invalid")]
    InvalidStoredDocument(#[from] serde_json::Error),
    #[error("bundle was imported before canonical export was supported: {0}")]
    LegacyBundleNotExportable(String),
}

impl Store {
    /// Opens a local `SQLite` database and applies the idempotent v1 schema.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the directory, database, pragma, or migration cannot be
    /// created or applied.
    pub fn open_and_migrate(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| {
                StoreError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
            })?;
        }
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS bundles (
                bundle_id TEXT PRIMARY KEY,
                schema_version TEXT NOT NULL,
                created_at TEXT NOT NULL,
                canonical_bundle_json TEXT,
                imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS events (
                bundle_id TEXT NOT NULL,
                event_id TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                canonical_json TEXT NOT NULL,
                PRIMARY KEY (bundle_id, event_id),
                UNIQUE (bundle_id, sequence),
                FOREIGN KEY (bundle_id) REFERENCES bundles(bundle_id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS artifacts (
                sha256 TEXT PRIMARY KEY,
                media_type TEXT NOT NULL,
                byte_length INTEGER NOT NULL,
                bytes BLOB NOT NULL,
                uploaded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        let has_canonical_bundle = {
            let mut statement = connection.prepare("PRAGMA table_info(bundles)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            columns
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .any(|name| name == "canonical_bundle_json")
        };
        if !has_canonical_bundle {
            connection.execute(
                "ALTER TABLE bundles ADD COLUMN canonical_bundle_json TEXT",
                [],
            )?;
        }
        Ok(Self { connection })
    }

    /// Validates and atomically imports a canonical bundle.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Conflict`] when the bundle already exists, or another
    /// [`StoreError`] when validation, serialization, or the transaction fails.
    pub fn import_bundle(&mut self, bundle: &Bundle) -> Result<(), StoreError> {
        bundle.validate()?;
        let canonical_bundle_json = serde_json::to_string(bundle)?;
        let transaction = self.connection.transaction()?;
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO bundles (bundle_id, schema_version, created_at, canonical_bundle_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                bundle.bundle_id,
                bundle.schema_version,
                bundle.created_at,
                canonical_bundle_json
            ],
        )?;
        if inserted == 0 {
            return Err(StoreError::Conflict(bundle.bundle_id.clone()));
        }

        {
            let mut statement = transaction.prepare(
                "INSERT INTO events (bundle_id, event_id, sequence, canonical_json) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for event in &bundle.events {
                statement.execute(params![
                    bundle.bundle_id,
                    event.event_id,
                    event.sequence,
                    serde_json::to_string(event)?,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Exports the complete canonical bundle document stored at import time.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] for an unknown bundle,
    /// [`StoreError::LegacyBundleNotExportable`] when the record predates canonical document
    /// storage, or another [`StoreError`] when reading or validating the document fails.
    pub fn export_bundle(&self, bundle_id: &str) -> Result<Bundle, StoreError> {
        let canonical_json = self
            .connection
            .query_row(
                "SELECT canonical_bundle_json FROM bundles WHERE bundle_id = ?1",
                [bundle_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound(bundle_id.to_owned()))?
            .ok_or_else(|| StoreError::LegacyBundleNotExportable(bundle_id.to_owned()))?;
        let bundle: Bundle = serde_json::from_str(&canonical_json)?;
        bundle.validate()?;
        Ok(bundle)
    }

    /// Reads a bundle and its canonical events in sequence order.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] for an unknown bundle, or another [`StoreError`] when
    /// reading, deserializing, or validating stored events fails.
    pub fn timeline(&self, bundle_id: &str) -> Result<Bundle, StoreError> {
        match self.export_bundle(bundle_id) {
            Ok(bundle) => return Ok(bundle),
            Err(StoreError::LegacyBundleNotExportable(_)) => {}
            Err(error) => return Err(error),
        }

        let metadata = self
            .connection
            .query_row(
                "SELECT schema_version, created_at FROM bundles WHERE bundle_id = ?1",
                [bundle_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::NotFound(bundle_id.to_owned()))?;

        let mut statement = self.connection.prepare(
            "SELECT canonical_json FROM events WHERE bundle_id = ?1 ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map([bundle_id], |row| row.get::<_, String>(0))?;
        let events = rows
            .map(|row| {
                let canonical_json = row?;
                serde_json::from_str(&canonical_json).map_err(StoreError::from)
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        let bundle = Bundle {
            schema_version: metadata.0,
            bundle_id: bundle_id.to_owned(),
            created_at: metadata.1,
            capture_policy: reprospan_core::CapturePolicy {
                mode: "metadata_only".to_owned(),
                redacted: true,
            },
            events,
            artifacts: Vec::new(),
        };
        bundle.validate()?;
        Ok(bundle)
    }

    /// Returns basic metadata for every imported bundle, newest first.
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] if the underlying query fails.
    pub fn list_bundles(&self) -> Result<Vec<BundleListItem>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT bundle_id, schema_version, created_at, imported_at FROM bundles ORDER BY imported_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(BundleListItem {
                bundle_id: row.get(0)?,
                schema_version: row.get(1)?,
                created_at: row.get(2)?,
                imported_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    /// Stores raw artifact bytes keyed by their `sha256` digest.
    ///
    /// Re-uploading the same digest is a no-op.
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] if the underlying database operation fails.
    pub fn store_artifact(
        &self,
        sha256: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        let byte_length: i64 = bytes.len().try_into().unwrap_or(i64::MAX);
        self.connection.execute(
            "INSERT OR IGNORE INTO artifacts (sha256, media_type, byte_length, bytes) VALUES (?1, ?2, ?3, ?4)",
            params![sha256, media_type, byte_length, bytes],
        )?;
        Ok(())
    }

    /// Reads artifact bytes for a given `sha256` digest, if stored.
    ///
    /// # Errors
    ///
    /// Returns a [`StoreError`] if the underlying database operation fails.
    pub fn artifact_bytes(&self, sha256: &str) -> Result<Option<Vec<u8>>, StoreError> {
        self.connection
            .query_row(
                "SELECT bytes FROM artifacts WHERE sha256 = ?1",
                [sha256],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(StoreError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/failed-tool-run.bundle.json"
    ));

    fn fixture() -> Bundle {
        serde_json::from_str(BUNDLE).expect("bundle fixture should deserialize")
    }

    #[test]
    fn imports_and_reads_timeline_in_sequence() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let mut store = Store::open_and_migrate(directory.path().join("store.sqlite"))
            .expect("store should open");
        let bundle = fixture();

        store.import_bundle(&bundle).expect("import should succeed");
        let timeline = store
            .timeline(&bundle.bundle_id)
            .expect("timeline should exist");
        let exported = store
            .export_bundle(&bundle.bundle_id)
            .expect("bundle should export");

        assert_eq!(timeline, bundle);
        assert_eq!(exported, bundle);
    }

    #[test]
    fn migrates_legacy_bundle_for_timeline_but_not_export() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let path = directory.path().join("legacy.sqlite");
        let bundle = fixture();
        {
            let connection = Connection::open(&path).expect("legacy database should open");
            connection
                .execute_batch(
                    "
                    CREATE TABLE bundles (
                        bundle_id TEXT PRIMARY KEY,
                        schema_version TEXT NOT NULL,
                        created_at TEXT NOT NULL,
                        imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    CREATE TABLE events (
                        bundle_id TEXT NOT NULL,
                        event_id TEXT NOT NULL,
                        sequence INTEGER NOT NULL,
                        canonical_json TEXT NOT NULL,
                        PRIMARY KEY (bundle_id, event_id),
                        UNIQUE (bundle_id, sequence)
                    );
                    ",
                )
                .unwrap();
            connection
                .execute(
                    "INSERT INTO bundles (bundle_id, schema_version, created_at) VALUES (?1, ?2, ?3)",
                    params![bundle.bundle_id, bundle.schema_version, bundle.created_at],
                )
                .unwrap();
            for event in &bundle.events {
                connection
                    .execute(
                        "INSERT INTO events (bundle_id, event_id, sequence, canonical_json) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            bundle.bundle_id,
                            event.event_id,
                            event.sequence,
                            serde_json::to_string(event).unwrap()
                        ],
                    )
                    .unwrap();
            }
        }

        let store = Store::open_and_migrate(&path).expect("legacy store should migrate");
        assert_eq!(
            store.timeline(&bundle.bundle_id).unwrap().events,
            bundle.events
        );
        assert!(matches!(
            store.export_bundle(&bundle.bundle_id),
            Err(StoreError::LegacyBundleNotExportable(_))
        ));
        Store::open_and_migrate(&path).expect("migration should be idempotent");
    }

    #[test]
    fn duplicate_bundle_is_a_conflict() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let mut store = Store::open_and_migrate(directory.path().join("store.sqlite"))
            .expect("store should open");
        let bundle = fixture();

        store
            .import_bundle(&bundle)
            .expect("first import should work");
        assert!(matches!(
            store.import_bundle(&bundle),
            Err(StoreError::Conflict(_))
        ));
    }

    #[test]
    fn unknown_bundle_is_not_found() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let store = Store::open_and_migrate(directory.path().join("store.sqlite"))
            .expect("store should open");

        assert!(matches!(
            store.timeline("missing"),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn stores_and_retrieves_artifact_bytes() {
        let directory = tempfile::tempdir().expect("temp directory should be created");
        let store = Store::open_and_migrate(directory.path().join("store.sqlite"))
            .expect("store should open");

        let sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let bytes = b"hello artifact";
        store
            .store_artifact(sha256, "text/plain", bytes)
            .expect("store should succeed");
        assert_eq!(
            store.artifact_bytes(sha256).expect("should exist"),
            Some(bytes.to_vec()),
        );
        assert!(
            store
                .artifact_bytes("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                .expect("query should succeed")
                .is_none()
        );

        // Re-upload is idempotent.
        store
            .store_artifact(sha256, "text/plain", bytes)
            .expect("re-store should succeed");
    }
}
