//! Operator builders for each supported OpenDAL service.
//!
//! This module used to be a single-file implementation. It is now split into
//! per-service files to make adding new backends easier and to keep each
//! builder small and well-documented. Each backend file exposes a
//! `build_<scheme>(cfg: &RemoteConfig) -> Result<Operator>` function.

use crate::config::RemoteConfig;
use anyhow::{Result, bail};
use opendal::{Operator, layers::LoggingLayer};
use tracing::debug;

mod azblob;
mod fs;
mod gcs;
mod gdrive;
mod memory;
mod s3;

/// Build a configured [`Operator`] for the remote.
pub async fn build_operator(cfg: &RemoteConfig) -> Result<Operator> {
    debug!("scheme={} root={}", cfg.scheme, cfg.root);

    let op = match cfg.scheme.as_str() {
        "s3" => s3::build_s3(cfg)?,
        "gcs" => gcs::build_gcs(cfg)?,
        "azblob" => azblob::build_azblob(cfg)?,
        "gdrive" => gdrive::build_gdrive(cfg)?,
        "fs" => fs::build_fs(cfg)?,
        "memory" => memory::build_memory()?,
        other => bail!(
            "Unsupported scheme '{other}'. Supported: s3, gcs, azblob, gdrive, fs, memory.\n\
             Enable the matching 'services-<name>' feature and add a branch in operator.rs."
        ),
    };

    Ok(op.layer(LoggingLayer::default()))
}

// No helper traits required — per-backend builders return `Result<Operator>` directly.
