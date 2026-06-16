//! Google Drive backend builder for opendal operator.
//!
//! Contains `build_gdrive` which constructs an operator for Google Drive.
//!
//! Optional params: `client-id`, `client-secret`, `refresh-token`, `access-token`.
//!
//! # Configuration
//!
//! Configuration is provided via environment variables:
//!
//! ```bash
//! export OPENDAL_GDRIVE_CLIENT_ID=my-client-id
//! export OPENDAL_GDRIVE_CLIENT_SECRET=my-client-secret
//! export OPENDAL_GDRIVE_REFRESH_TOKEN=my-refresh-token
//! export OPENDAL_GDRIVE_ACCESS_TOKEN=my-access-token
//! ```

use crate::config::RemoteConfig;
use anyhow::Result;
use opendal::Operator;

/// Build a Google Drive `Operator` from the `RemoteConfig`.
pub fn build_gdrive(cfg: &RemoteConfig) -> Result<Operator> {
    use opendal::services::Gdrive;
    use tracing::debug;

    debug!("building Gdrive operator for root={}", cfg.root);

    let mut b = Gdrive::default();
    b = b.root(&cfg.root);

    if let Some(v) = cfg.params.get("client-id") {
        b = b.client_id(v);
    }
    if let Some(v) = cfg.params.get("client-secret") {
        b = b.client_secret(v);
    }
    if let Some(v) = cfg.params.get("refresh-token") {
        b = b.refresh_token(v);
    }
    if let Some(v) = cfg.params.get("access-token") {
        b = b.access_token(v);
    }

    Ok(opendal::Operator::new(b)?.finish())
}
