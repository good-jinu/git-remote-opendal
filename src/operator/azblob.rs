//! Azure Blob backend builder for opendal operator.
//!
//! Contains `build_azblob` which constructs an operator for Azure Blob Storage.
//!
//! Required param: `container`, supplied either as the first path segment of
//! the remote URL (e.g. `opendal://azblob/my-container/repos/myrepo`) or via
//! `OPENDAL_AZBLOB_CONTAINER`. Optional: `account-name`, `account-key`,
//! `endpoint`.
//!
//! # Configuration
//!
//! Configuration is provided via environment variables:
//!
//! ```bash
//! export OPENDAL_AZBLOB_CONTAINER=my-git-container
//! export OPENDAL_AZBLOB_ACCOUNT_NAME=account_name
//! export OPENDAL_AZBLOB_ACCOUNT_KEY=account_key
//! export OPENDAL_AZBLOB_ENDPOINT=https://account_name.blob.core.windows.net
//! ```

use crate::config::RemoteConfig;
use anyhow::Result;
use opendal::Operator;

/// Build an Azblob `Operator` from the `RemoteConfig`.
pub fn build_azblob(cfg: &RemoteConfig) -> Result<Operator> {
    use anyhow::anyhow;
    use opendal::services::Azblob;
    use tracing::debug;

    let container = cfg.params.get("container").ok_or_else(|| {
        anyhow!(
            "Azure Blob requires a container, e.g. opendal://azblob/my-container/path or OPENDAL_AZBLOB_CONTAINER.\n\
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
