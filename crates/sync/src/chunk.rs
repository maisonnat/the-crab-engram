use std::io::{Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

use engram_core::EngramError;
use engram_store::{ExportData, Storage};

/// Chunk manifest for sync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkManifest {
    pub chunks: Vec<ChunkEntry>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkEntry {
    pub id: String,       // SHA-256 of chunk content
    pub filename: String, // chunk_001.jsonl.gz
    pub size: usize,      // uncompressed bytes
    pub observation_count: usize,
}

/// Export data as JSONL.gz chunks (Go-compatible format).
pub fn export_chunks(
    store: &dyn Storage,
    project: Option<&str>,
    output_dir: &Path,
) -> Result<ChunkManifest, EngramError> {
    std::fs::create_dir_all(output_dir)
        .map_err(|e| EngramError::Sync(format!("failed to create output dir: {e}")))?;

    let data = store.export(project)?;
    let mut chunks = Vec::new();

    // Chunk 1: observations (JSONL format - one JSON object per line)
    let obs_jsonl = to_jsonl(&data.observations)?;
    let obs_chunk = compress_chunk(&obs_jsonl)?;
    let obs_id = hash_chunk(&obs_jsonl);
    let obs_filename = format!("observations_{}.jsonl.gz", &obs_id[..8]);
    std::fs::write(output_dir.join(&obs_filename), &obs_chunk)
        .map_err(|e| EngramError::Sync(format!("failed to write chunk: {e}")))?;

    chunks.push(ChunkEntry {
        id: obs_id,
        filename: obs_filename,
        size: obs_jsonl.len(),
        observation_count: data.observations.len(),
    });

    // Chunk 2: sessions
    let sess_jsonl = to_jsonl(&data.sessions)?;
    let sess_chunk = compress_chunk(&sess_jsonl)?;
    let sess_id = hash_chunk(&sess_jsonl);
    let sess_filename = format!("sessions_{}.jsonl.gz", &sess_id[..8]);
    std::fs::write(output_dir.join(&sess_filename), &sess_chunk)
        .map_err(|e| EngramError::Sync(format!("failed to write chunk: {e}")))?;

    chunks.push(ChunkEntry {
        id: sess_id,
        filename: sess_filename,
        size: sess_jsonl.len(),
        observation_count: 0,
    });

    // Write manifest
    let manifest = ChunkManifest {
        chunks,
        created_at: chrono::Utc::now(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(output_dir.join("manifest.json"), &manifest_json)
        .map_err(|e| EngramError::Sync(format!("failed to write manifest: {e}")))?;

    Ok(manifest)
}

/// Import chunks from a directory.
pub fn import_chunks(
    store: &dyn Storage,
    input_dir: &Path,
) -> Result<engram_store::ImportResult, EngramError> {
    let manifest_path = input_dir.join("manifest.json");
    let manifest_json = std::fs::read_to_string(&manifest_path)
        .map_err(|e| EngramError::Sync(format!("failed to read manifest: {e}")))?;
    let manifest: ChunkManifest = serde_json::from_str(&manifest_json)?;

    let mut all_observations = Vec::new();
    let mut all_sessions = Vec::new();

    for chunk_entry in &manifest.chunks {
        let chunk_path = input_dir.join(&chunk_entry.filename);
        let compressed = std::fs::read(&chunk_path)
            .map_err(|e| EngramError::Sync(format!("failed to read chunk: {e}")))?;
        let jsonl = decompress_chunk(&compressed)?;

        if chunk_entry.filename.starts_with("observations") {
            for line in jsonl.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let obs: engram_core::Observation = serde_json::from_str(line)?;
                all_observations.push(obs);
            }
        } else if chunk_entry.filename.starts_with("sessions") {
            for line in jsonl.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let session: engram_core::Session = serde_json::from_str(line)?;
                all_sessions.push(session);
            }
        }
    }

    let data = ExportData {
        observations: all_observations,
        sessions: all_sessions,
        prompts: Vec::new(),
        edges: Vec::new(),
    };

    store.import(&data)
}

// ── Helpers ───────────────────────────────────────────────────────

fn to_jsonl<T: serde::Serialize>(items: &[T]) -> Result<Vec<u8>, EngramError> {
    let mut buf = Vec::new();
    for item in items {
        let json = serde_json::to_vec(item)?;
        buf.extend_from_slice(&json);
        buf.push(b'\n');
    }
    Ok(buf)
}

fn compress_chunk(data: &[u8]) -> Result<Vec<u8>, EngramError> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| EngramError::Sync(format!("compression failed: {e}")))?;
    encoder
        .finish()
        .map_err(|e| EngramError::Sync(format!("compression failed: {e}")))
}

fn decompress_chunk(data: &[u8]) -> Result<String, EngramError> {
    let mut decoder = GzDecoder::new(data);
    let mut result = String::new();
    decoder
        .read_to_string(&mut result)
        .map_err(|e| EngramError::Sync(format!("decompression failed: {e}")))?;
    Ok(result)
}

fn hash_chunk(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::ObservationType;
    use engram_store::{AddObservationParams, SqliteStore};

    #[test]
    fn export_import_roundtrip() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: engram_core::Scope::Project,
                title: "Use SQLite".into(),
                content: "For local storage".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        let output_dir = std::env::temp_dir().join("engram-test-chunks");
        let manifest = export_chunks(&store, None, &output_dir).unwrap();

        assert!(!manifest.chunks.is_empty());
        assert!(output_dir.join("manifest.json").exists());

        // Import into fresh store
        let store2 = SqliteStore::in_memory().unwrap();
        let result = import_chunks(&store2, &output_dir).unwrap();
        assert_eq!(result.observations_imported, 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(&output_dir);
    }

    #[test]
    fn compress_decompress_roundtrip() {
        let data = b"hello world test data";
        let compressed = compress_chunk(data).unwrap();
        let decompressed = decompress_chunk(&compressed).unwrap();
        assert_eq!(decompressed.as_bytes(), data);
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_chunk(b"test");
        let h2 = hash_chunk(b"test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn jsonl_format() {
        let items = vec!["a", "b", "c"];
        let jsonl = to_jsonl(&items).unwrap();
        let lines: Vec<&str> = std::str::from_utf8(&jsonl).unwrap().lines().collect();
        assert_eq!(lines.len(), 3);
    }
}
