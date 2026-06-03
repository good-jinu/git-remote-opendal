//! Configuration parsing for git-remote-opendal.
//!
//! ## Parameter resolution order (highest → lowest priority)
//!
//! 1. **git config** — `remote.<name>.opendal<Scheme><Param>` keys in any
//!    git config file (local `.git/config`, global `~/.gitconfig`, system).
//!    These are read via `git config --get <key>` so git's full config
//!    resolution chain applies automatically.
//!
//! 2. **Environment variables** — `OPENDAL_<SCHEME>_<KEY>=<value>`, mirroring
//!    the convention used by OpenDAL's own CLI (`oli`).  Useful in CI/CD or
//!    when you don't want to touch any config file.
//!
//! Credentials (keys, passwords) belong in env vars or a git credential
//! helper — not in git config where they could be committed accidentally.
//!
//! ## git config key naming convention
//!
//! Git config key names are camelCase, scheme-prefixed:
//!
//!   `opendal<Scheme><Param>`  →  e.g. `opendalS3Bucket`, `opendalGcsBucket`
//!
//! The param segment maps to the same lowercase-hyphenated keys used internally
//! (`bucket`, `region`, `endpoint`, `access-key-id`, …).  The mapping is:
//!   camelCase suffix → lowercase-hyphenated:  `AccessKeyId` → `access-key-id`
//!
//! ## Environment Variable Reference
//!
//! You can configure any backend using environment variables following the
//! `OPENDAL_<SCHEME>_<KEY>` pattern.
//!
//! | Backend | Variable | Description |
//! | :--- | :--- | :--- |
//! | **S3** | `OPENDAL_S3_BUCKET` | **Required.** The S3 bucket name. |
//! | | `OPENDAL_S3_REGION` | AWS region (e.g., `us-east-1`). |
//! | | `OPENDAL_S3_ENDPOINT` | Custom endpoint (e.g., MinIO, B2). |
//! | | `OPENDAL_S3_ACCESS_KEY_ID` | AWS Access Key. |
//! | | `OPENDAL_S3_SECRET_ACCESS_KEY`| AWS Secret Key. |
//! | **GCS** | `OPENDAL_GCS_BUCKET` | **Required.** The GCS bucket name. |
//! | | `OPENDAL_GCS_CREDENTIAL` | Raw JSON credential string. |
//! | | `OPENDAL_GCS_CREDENTIAL_PATH` | Path to service account JSON file. |
//! | | `OPENDAL_GCS_ENDPOINT` | Custom GCS-compatible endpoint. |
//! | **Azblob** | `OPENDAL_AZBLOB_CONTAINER` | **Required.** Azure container name. |
//! | | `OPENDAL_AZBLOB_ACCOUNT_NAME` | Storage account name. |
//! | | `OPENDAL_AZBLOB_ACCOUNT_KEY` | Storage account key. |
//! | | `OPENDAL_AZBLOB_ENDPOINT` | Custom Azure endpoint. |
//! | **Gdrive** | `OPENDAL_GDRIVE_CLIENT_ID` | OAuth2 Client ID. |
//! | | `OPENDAL_GDRIVE_CLIENT_SECRET`| OAuth2 Client Secret. |
//! | | `OPENDAL_GDRIVE_REFRESH_TOKEN`| OAuth2 Refresh Token. |
//! | | `OPENDAL_GDRIVE_ACCESS_TOKEN` | Temporary Access Token. |
//!
//! ## Git Config Reference
//!
//! You can also store these in your git configuration (e.g., `.git/config`)
//! using the key pattern `remote.<name>.opendal<Scheme><Param>`.
//!
//! | Backend | Git Config Key | Description |
//! | :--- | :--- | :--- |
//! | **S3** | `opendalS3Bucket` | **Required.** The S3 bucket name. |
//! | | `opendalS3Region` | AWS region. |
//! | | `opendalS3Endpoint` | Custom endpoint. |
//! | | `opendalS3AccessKeyId` | AWS Access Key. |
//! | | `opendalS3SecretAccessKey`| AWS Secret Key. |
//! | **GCS** | `opendalGcsBucket` | **Required.** The GCS bucket name. |
//! | | `opendalGcsCredential` | Raw JSON credential string. |
//! | | `opendalGcsCredentialPath` | Path to service account JSON file. |
//! | | `opendalGcsEndpoint` | Custom GCS-compatible endpoint. |
//! | **Azblob** | `opendalAzblobContainer` | **Required.** Azure container name. |
//! | | `opendalAzblobAccountName` | Storage account name. |
//! | | `opendalAzblobAccountKey` | Storage account key. |
//! | | `opendalAzblobEndpoint` | Custom Azure endpoint. |
//! | **Gdrive** | `opendalGdriveClientId` | OAuth2 Client ID. |
//! | | `opendalGdriveClientSecret`| OAuth2 Client Secret. |
//! | | `opendalGdriveRefreshToken`| OAuth2 Refresh Token. |
//! | | `opendalGdriveAccessToken` | Temporary Access Token. |
//!
//! ## Example .git/config
//!
//! ```ini
//! [remote "origin"]
//!     url = opendal://s3/my-git-repos/myrepo.git
//!     opendalS3Bucket = my-git-bucket
//!     opendalS3Region = us-east-1
//!     opendalS3Endpoint = https://s3.us-west-004.backblazeb2.com
//!
//! [remote "gcs-backup"]
//!     url = opendal://gcs/backups/myrepo.git
//!     opendalGcsBucket = my-gcs-bucket
//!
//! [remote "azure"]
//!     url = opendal://azblob/repos/myrepo.git
//!     opendalAzblobContainer = my-container
//!     opendalAzblobAccountName = myaccount
//! ```
//!
//! Set via CLI:
//! ```sh
//! git config remote.origin.opendalS3Bucket my-git-bucket
//! git config remote.origin.opendalS3Region us-east-1
//! ```

use anyhow::{Result, bail};
use std::collections::HashMap;
use std::process::Command;
use tracing::debug;

/// Parsed remote configuration.
#[derive(Debug, Clone)]
pub struct RemoteConfig {
    /// OpenDAL scheme string, e.g. "s3", "gcs", "azblob", "fs".
    pub scheme: String,

    /// Root path inside the storage backend for this repository.
    pub root: String,

    /// Merged backend parameters.
    ///
    /// Keys are lowercase-hyphenated (`bucket`, `region`, `access-key-id`).
    /// Values come from git config first, env vars as fallback.
    pub params: HashMap<String, String>,

    /// The original remote name (may be empty for direct invocations).
    #[allow(dead_code)]
    pub remote_name: String,
}

impl RemoteConfig {
    /// Parse configuration from the remote URL, git config, and env vars.
    pub fn from_url_and_env(url: &str, remote_name: &str) -> Result<Self> {
        let (scheme, root) = parse_url(url)?;

        // Layer 2 (low priority): environment variables OPENDAL_<SCHEME>_<KEY>
        let env_params = collect_env_params(&scheme);

        // Layer 1 (high priority): git config remote.<name>.opendal<Scheme><Param>
        let git_params = if remote_name.is_empty() {
            HashMap::new()
        } else {
            collect_git_params(remote_name, &scheme)
        };

        // Merge: git config wins over env vars.
        let mut params = env_params;
        params.extend(git_params);

        debug!(
            "config resolved: scheme={scheme}, root={root}, params={:?}",
            params.keys().collect::<Vec<_>>()
        );

        Ok(RemoteConfig {
            scheme,
            root,
            params,
            remote_name: remote_name.to_string(),
        })
    }
}

// ─── URL parsing ─────────────────────────────────────────────────────────────

fn parse_url(url: &str) -> Result<(String, String)> {
    let rest = url
        .strip_prefix("opendal://")
        .ok_or_else(|| anyhow::anyhow!("URL must start with 'opendal://', got: {}", url))?;

    let (scheme, root_suffix) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };

    if scheme.is_empty() {
        bail!("Scheme must not be empty in URL: {}", url);
    }

    let root = if root_suffix.is_empty() {
        "/".to_string()
    } else {
        root_suffix.to_string()
    };

    Ok((scheme.to_string(), root))
}

// ─── Environment variable collection ─────────────────────────────────────────

/// Collect `OPENDAL_<SCHEME>_<KEY>` vars and return them as lowercase-hyphenated keys.
fn collect_env_params(scheme: &str) -> HashMap<String, String> {
    let prefix = format!("OPENDAL_{}_", scheme.to_uppercase());
    std::env::vars()
        .filter_map(|(k, v)| {
            k.strip_prefix(&prefix)
                .map(|suffix| (suffix.to_lowercase().replace('_', "-"), v))
        })
        .collect()
}

// ─── git config collection ────────────────────────────────────────────────────

/// Query git config for all keys matching `remote.<name>.opendal<Scheme>*`.
///
/// Uses `git config --get-regexp` to fetch all matching keys in one call,
/// then maps each camelCase suffix back to lowercase-hyphenated form.
///
/// Never fails — a missing git or absent keys returns an empty map so the
/// helper can still operate via env vars alone.
fn collect_git_params(remote_name: &str, scheme: &str) -> HashMap<String, String> {
    // The git config section prefix for this remote, e.g. "remote.origin."
    let section = format!("remote.{}.", remote_name);

    // The key prefix within the section, e.g. "opendalS3" for scheme "s3".
    // We build the camelCase scheme component: "s3" → "S3", "azblob" → "Azblob".
    let scheme_camel = to_camel_case_first(scheme);
    let key_prefix = format!("opendal{}", scheme_camel);

    // Run: git config --get-regexp "^remote\.<name>\.opendal<Scheme>"
    // This returns lines like:
    //   remote.origin.opendalS3Bucket my-bucket
    //   remote.origin.opendalS3Region us-east-1
    let pattern = format!("^{}{}", regex_escape(&section), regex_escape(&key_prefix));

    let output = Command::new("git")
        .args(["config", "--get-regexp", &pattern])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            debug!("git config subprocess failed: {}", e);
            return HashMap::new();
        }
    };

    // Exit code 1 means no keys matched — not an error.
    if !output.status.success() && output.status.code() != Some(1) {
        debug!(
            "git config --get-regexp exited with {:?}",
            output.status.code()
        );
        return HashMap::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let full_prefix = format!("{}{}", section, key_prefix).to_lowercase();

    let mut map = HashMap::new();

    for line in stdout.lines() {
        // Each line: "remote.origin.opendalS3Bucket my-bucket"
        // git config normalises key names to lowercase.
        let mut parts = line.splitn(2, ' ');
        let key = match parts.next() {
            Some(k) => k,
            None => continue,
        };
        let val = match parts.next() {
            Some(v) => v.trim(),
            None => continue,
        };

        // Strip the "remote.<name>.opendal<scheme>" prefix (already lowercased).
        let param_suffix = match key.strip_prefix(&full_prefix) {
            Some(s) => s,
            None => continue,
        };

        // "bucket" or "accountname" → lowercase-hyphenated param key.
        // We stored the camelCase suffix in git config, git lowercased it,
        // so we just need to reverse the underscore→hyphen substitution.
        // e.g. "accountname" stays "accountname" but we know the canonical
        // form from our own table.
        let param_key = normalize_git_param_key(param_suffix);

        debug!("git config: {}={}", param_key, val);
        map.insert(param_key, val.to_string());
    }

    map
}

/// Map the lowercased suffix from git config back to our canonical
/// lowercase-hyphenated param key.
///
/// git config lowercases everything, so `AccessKeyId` → `accesskeyid`.
/// We keep a lookup table of known params to restore the hyphen positions.
fn normalize_git_param_key(lowercased_suffix: &str) -> String {
    // Known multi-word params that need hyphens restored.
    // Format: (git-lowercased, canonical-hyphenated)
    const KNOWN: &[(&str, &str)] = &[
        // S3
        ("accesskeyid", "access-key-id"),
        ("secretaccesskey", "secret-access-key"),
        // Azure Blob
        ("accountname", "account-name"),
        ("accountkey", "account-key"),
        // GCS
        ("credentialpath", "credential-path"),
        // Google Drive
        ("clientid", "client-id"),
        ("clientsecret", "client-secret"),
        ("refreshtoken", "refresh-token"),
        ("accesstoken", "access-token"),
        // Generic
        // single-word params (bucket, region, endpoint, container, credential)
        // need no mapping — they're already correct as-is.
    ];

    for (from, to) in KNOWN {
        if lowercased_suffix == *from {
            return to.to_string();
        }
    }

    // Single-word params (bucket, region, endpoint, …) pass through unchanged.
    lowercased_suffix.to_string()
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/// Capitalise only the first character of a string.
/// "s3" → "S3", "azblob" → "Azblob", "gcs" → "Gcs"
fn to_camel_case_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Escape the characters that are special in the git config regex dialect
/// (POSIX Basic Regular Expressions).
/// Special characters in BRE: . * [ ^ $ \
fn regex_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '.' | '*' | '[' | '^' | '$' | '\\' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn parse_s3_url() {
        let (scheme, root) = parse_url("opendal://s3/repos/myrepo.git").unwrap();
        assert_eq!(scheme, "s3");
        assert_eq!(root, "/repos/myrepo.git");
    }

    #[test]
    fn parse_fs_url() {
        let (scheme, root) = parse_url("opendal://fs/tmp/repos/myrepo.git").unwrap();
        assert_eq!(scheme, "fs");
        assert_eq!(root, "/tmp/repos/myrepo.git");
    }

    #[test]
    fn parse_scheme_only() {
        let (scheme, root) = parse_url("opendal://memory").unwrap();
        assert_eq!(scheme, "memory");
        assert_eq!(root, "/");
    }

    #[test]
    fn reject_non_opendal_url() {
        assert!(parse_url("s3://bucket/path").is_err());
    }

    #[test]
    fn camel_first() {
        assert_eq!(to_camel_case_first("s3"), "S3");
        assert_eq!(to_camel_case_first("azblob"), "Azblob");
        assert_eq!(to_camel_case_first("gcs"), "Gcs");
    }

    #[test]
    fn normalize_known_keys() {
        assert_eq!(normalize_git_param_key("accesskeyid"), "access-key-id");
        assert_eq!(
            normalize_git_param_key("secretaccesskey"),
            "secret-access-key"
        );
        assert_eq!(normalize_git_param_key("accountname"), "account-name");
        assert_eq!(normalize_git_param_key("bucket"), "bucket");
        assert_eq!(normalize_git_param_key("region"), "region");
    }

    #[test]
    #[serial]
    fn env_params_collected() {
        // Temporarily set a fake env var.
        unsafe {
            std::env::set_var("OPENDAL_S3_BUCKET", "test-bucket");
        }
        let params = collect_env_params("s3");
        assert_eq!(
            params.get("bucket").map(String::as_str),
            Some("test-bucket")
        );
        unsafe {
            std::env::remove_var("OPENDAL_S3_BUCKET");
        }
    }

    #[test]
    fn regex_escape_special_chars() {
        assert_eq!(regex_escape("remote.origin."), r"remote\.origin\.");
        assert_eq!(regex_escape(r"a.b*c[d^e$f\g"), r"a\.b\*c\[d\^e\$f\\g");
    }
}
