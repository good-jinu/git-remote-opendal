//! S3 backend builder for opendal operator.
//!
//! This file contains the `build_s3` function which constructs an
//! `opendal::Operator` configured for S3-compatible services (AWS S3,
//! MinIO, Cloudflare R2, etc.).
//!
//! Required parameter (env or git-config): `bucket` (canonical key
//! `OPENDAL_S3_BUCKET` / git `opendalS3Bucket`). Optional params:
//! `region`, `endpoint`, `access-key-id`, `secret-access-key`.
//!
//! # Configuration
//!
//! Configuration can be provided via environment variables or git config:
//!
//! ```bash
//! # Via environment variable
//! export OPENDAL_S3_BUCKET=my-git-bucket
//!
//! # Via git config
//! git config remote.<name>.opendalS3Bucket my-git-bucket
//! ```

use crate::config::RemoteConfig;
use anyhow::Result;
use opendal::Operator;

/// Build an S3 `Operator` from the merged `RemoteConfig`.
///
/// Examples of config keys used:
/// - `bucket` (required)
/// - `region`
/// - `endpoint`
/// - `access-key-id`
/// - `secret-access-key`
pub fn build_s3(cfg: &RemoteConfig) -> Result<Operator> {
    use anyhow::anyhow;
    use opendal::services::S3;
    use tracing::debug;

    let bucket = cfg.params.get("bucket").ok_or_else(|| {
        anyhow!(
            "S3 requires OPENDAL_S3_BUCKET.\n\
             Example: export OPENDAL_S3_BUCKET=my-git-bucket"
        )
    })?;

    debug!("building S3 operator for bucket={}", bucket);

    let mut b = S3::default();
    b = b.bucket(bucket);
    b = b.root(&cfg.root);

    if let Some(v) = cfg.params.get("region") {
        b = b.region(v);
    }
    if let Some(v) = cfg.params.get("endpoint") {
        b = b.endpoint(v);
    }
    if let Some(v) = cfg.params.get("access-key-id") {
        b = b.access_key_id(v);
    }
    if let Some(v) = cfg.params.get("secret-access-key") {
        b = b.secret_access_key(v);
    }

    Ok(opendal::Operator::new(b)?.finish())
}
