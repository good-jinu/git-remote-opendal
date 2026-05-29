//! Azure Blob backend builder for opendal operator.
//!
//! Contains `build_azblob` which constructs an operator for Azure Blob Storage.
//!
//! Required param: `container` (`OPENDAL_AZBLOB_CONTAINER` / `opendalAzblobContainer`).
//! Optional: `account-name`, `account-key`, `endpoint`.
//!
//! # Configuration
//!
//! Configuration can be provided via environment variables or git config:
//!
//! ```bash
//! # Via environment variable
//! export OPENDAL_AZBLOB_CONTAINER=my-git-container
//!
//! # Via git config
//! git config opendal.azblob.container my-git-container
//! ```

use crate::config::RemoteConfig;
use anyhow::Result;
use opendal::Operator;

/// Build an Azblob `Operator` from the merged `RemoteConfig`.
pub fn build_azblob(cfg: &RemoteConfig) -> Result<Operator> {
    use anyhow::anyhow;
    use opendal::services::Azblob;
    use tracing::debug;

    let container = cfg.params.get("container").ok_or_else(|| {
        anyhow!(
            "Azure Blob requires OPENDAL_AZBLOB_CONTAINER.\n\
             Example: export OPENDAL_AZBLOB_CONTAINER=my-git-container"
        )
    })?;

    debug!("building Azblob operator for container={}", container);

    let mut b = Azblob::default();
    b = b.container(container);
    b = b.root(&cfg.root);

    if let Some(v) = cfg.params.get("account-name") {
        b = b.account_name(v);
    }
    if let Some(v) = cfg.params.get("account-key") {
        b = b.account_key(v);
    }
    if let Some(v) = cfg.params.get("endpoint") {
        b = b.endpoint(v);
    }

    Ok(opendal::Operator::new(b)?.finish())
}
