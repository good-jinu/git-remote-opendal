//! Local filesystem backend builder for opendal operator.
//!
//! Contains `build_fs` which constructs a filesystem-backed operator. The
//! `root` part of the opendal URL is normalised to a single leading slash.

use crate::config::RemoteConfig;
use anyhow::Result;
use opendal::Operator;

/// Build a Fs `Operator` from the `RemoteConfig`.
pub fn build_fs(cfg: &RemoteConfig) -> Result<Operator> {
    use opendal::services::Fs;
    use tracing::debug;

    // Normalize root to a single leading slash for absolute paths.
    let root = normalize_fs_root(&cfg.root);
    debug!("building Fs operator for root={}", root);

    let mut b = Fs::default();
    b = b.root(&root);

    Ok(opendal::Operator::new(b)?.finish())
}

fn normalize_fs_root(root: &str) -> String {
    let trimmed = root.trim_start_matches('/');
    format!("/{}", trimmed)
}
