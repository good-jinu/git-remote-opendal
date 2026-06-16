//! GCS backend builder for opendal operator.
//!
//! Contains `build_gcs` which constructs an operator for Google Cloud Storage.
//!
//! Required param: `bucket` (`OPENDAL_GCS_BUCKET`). Optional:
//! `credential`, `credential-path`, `endpoint`.
//!
//! # Configuration
//!
//! Configuration is provided via environment variables:
//!
//! ```bash
//! export OPENDAL_GCS_BUCKET=my-git-bucket
//! export OPENDAL_GCS_CREDENTIAL=credential_json
//! export OPENDAL_GCS_CREDENTIAL_PATH=/path/to/credential
//! export OPENDAL_GCS_ENDPOINT=https://storage.googleapis.com
//! ```

use crate::config::RemoteConfig;
use anyhow::Result;
use opendal::Operator;

/// Build a GCS `Operator` from the `RemoteConfig`.
pub fn build_gcs(cfg: &RemoteConfig) -> Result<Operator> {
    use anyhow::anyhow;
    use opendal::services::Gcs;
    use tracing::debug;

    let bucket = cfg.params.get("bucket").ok_or_else(|| {
        anyhow!(
            "GCS requires OPENDAL_GCS_BUCKET.\n\
             Example: export OPENDAL_GCS_BUCKET=my-git-bucket"
        )
    })?;

    debug!("building GCS operator for bucket={}", bucket);

    let mut b = Gcs::default();
    b = b.bucket(bucket);
    b = b.root(&cfg.root);

    if let Some(v) = cfg.params.get("credential") {
        b = b.credential(v);
    }
    if let Some(v) = cfg.params.get("credential-path") {
        b = b.credential_path(v);
    }
    if let Some(v) = cfg.params.get("endpoint") {
        b = b.endpoint(v);
    }

    Ok(opendal::Operator::new(b)?.finish())
}
