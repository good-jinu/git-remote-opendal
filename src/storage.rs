//! High-level storage operations backed by an OpenDAL [`Operator`].
//!
//! Layout inside the repository root on the storage backend:
//!
//! ```text
//! <root>/
//!   info/
//!     refs.json          ← JSON map of { "refs/heads/main": "<sha1>", ... }
//!   objects/
//!     <timestamp>.bundle ← git bundle files (one per push, in chronological order)
//! ```
//!
//! ## Push flow
//! The helper receives a git fast-export stream, materialises objects via
//! `git fast-import`, creates a bundle with `git bundle create`, and uploads
//! the bundle.  `info/refs.json` is updated atomically to record the new
//! tip SHAs and the ordered list of bundle keys.
//!
//! ## Fetch flow
//! The helper downloads all bundles in the recorded order and applies each
//! via `git bundle unbundle`, producing the fast-import stream that git
//! expects.

use anyhow::{Context, Result};
use opendal::Operator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

const REFS_PATH: &str = "info/refs.json";

/// A map of ref name → SHA-1 hex string.
pub type RefMap = HashMap<String, String>;

/// Serialised form stored at `info/refs.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefsStore {
    /// ref name → SHA-1
    pub refs: RefMap,
    /// Ordered list of bundle object keys (relative paths under the root).
    /// Each successful push appends one entry.
    pub bundles: Vec<String>,
}

/// Storage abstraction scoped to a single git repository root.
pub struct Storage {
    op: Operator,
}

impl Storage {
    pub fn new(op: Operator) -> Self {
        Self { op }
    }

    // ─── Refs ────────────────────────────────────────────────────────────────

    /// Load the refs store, returning an empty store for a brand-new repository.
    pub async fn load_refs(&self) -> Result<RefsStore> {
        match self.op.read(REFS_PATH).await {
            Ok(buf) => {
                // opendal ≥0.46 returns a `Buffer` which implements Into<Bytes>.
                let bytes: bytes::Bytes = buf.to_bytes();
                let store: RefsStore = serde_json::from_slice(&bytes)
                    .context("refs.json is present but could not be parsed — file corrupted?")?;
                debug!(
                    "loaded {} refs, {} bundles",
                    store.refs.len(),
                    store.bundles.len()
                );
                Ok(store)
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => {
                debug!("refs.json not found — empty repository");
                Ok(RefsStore::default())
            }
            Err(e) => Err(e).context("Failed to read refs.json"),
        }
    }

    /// Persist the refs store.
    pub async fn save_refs(&self, store: &RefsStore) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(store).context("Failed to serialise refs.json")?;
        self.op
            .write(REFS_PATH, bytes)
            .await
            .context("Failed to write refs.json")?;
        info!(
            "refs saved: {} refs, {} bundles",
            store.refs.len(),
            store.bundles.len()
        );
        Ok(())
    }

    // ─── Bundles ─────────────────────────────────────────────────────────────

    /// Upload a bundle and return its storage key.
    pub async fn upload_bundle(&self, name: &str, data: Vec<u8>) -> Result<String> {
        let key = format!("objects/{}.bundle", name);
        debug!("uploading {} bytes → {}", data.len(), key);
        self.op
            .write(&key, data)
            .await
            .with_context(|| format!("Failed to upload bundle {}", key))?;
        info!("bundle uploaded: {}", key);
        Ok(key)
    }

    /// Download a bundle by its storage key.
    pub async fn download_bundle(&self, key: &str) -> Result<Vec<u8>> {
        debug!("downloading bundle: {}", key);
        let buf = self
            .op
            .read(key)
            .await
            .with_context(|| format!("Failed to download bundle {}", key))?;
        Ok(buf.to_bytes().to_vec())
    }
}
