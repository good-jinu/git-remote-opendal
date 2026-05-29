//! Core remote helper state machine.
//!
//! **Fetch flow** (git clone / git fetch):
//!   1. git sends `capabilities` → we reply `import\npush\nrefspec ...\n\n`
//!   2. git sends `list`         → we reply with our stored refs
//!   3. git sends `import <ref>` batches → we apply stored bundles via
//!      `git bundle unbundle` and emit a fast-import stream back to git.
//!
//! **Push flow** (git push):
//!   1. git sends `capabilities` → same
//!   2. git sends `list for-push` → we reply with stored refs
//!   3. git sends `push <src>:<dst>` (one per ref, terminated by blank line)
//!      → we create a git bundle from the local refs, upload it, update refs.json.

use crate::config::RemoteConfig;
use crate::protocol::{self, Command};
use crate::storage::{RefsStore, Storage};
use anyhow::{Context, Result, bail};
use opendal::Operator;
use std::io;
use std::process::{Command as SysCommand, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

const REFSPEC: &str = "refs/heads/*:refs/heads/*";

pub struct RemoteHelper {
    storage: Storage,
    #[allow(dead_code)]
    cfg: RemoteConfig,
}

impl RemoteHelper {
    pub fn new(op: Operator, cfg: RemoteConfig) -> Self {
        Self {
            storage: Storage::new(op),
            cfg,
        }
    }

    /// Main protocol loop — reads commands from git until EOF.
    pub async fn run(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();

        loop {
            let cmd = protocol::read_command(&mut stdin)?;
            match cmd {
                None => {
                    debug!("stdin closed — exiting");
                    break;
                }
                Some(Command::Blank) => {
                    debug!("received unexpected blank line — ignoring");
                }
                Some(Command::Capabilities) => {
                    self.handle_capabilities()?;
                }
                Some(Command::List) => {
                    self.handle_list(false).await?;
                }
                Some(Command::ListForPush) => {
                    self.handle_list(true).await?;
                }
                Some(Command::Import(first_ref)) => {
                    let mut refs = first_ref;
                    loop {
                        match protocol::read_command(&mut stdin)? {
                            Some(Command::Import(r)) => refs.extend(r),
                            Some(Command::Blank) => break,
                            Some(other) => {
                                bail!("Unexpected command during import batch: {:?}", other)
                            }
                            None => break,
                        }
                    }
                    self.handle_import(&refs).await?;
                }
                Some(Command::Push { src, dst }) => {
                    let mut pushes = vec![(src, dst)];
                    loop {
                        match protocol::read_command(&mut stdin)? {
                            Some(Command::Push { src, dst }) => pushes.push((src, dst)),
                            Some(Command::Blank) | None => break,
                            Some(other) => {
                                bail!("Unexpected command during push batch: {:?}", other)
                            }
                        }
                    }
                    self.handle_push(&pushes).await?;
                }
                Some(Command::Option(key, val)) => {
                    self.handle_option(&key, &val)?;
                }
                Some(Command::Unknown(cmd)) => {
                    warn!("Unknown command from git: '{}' — ignoring", cmd);
                }
            }
        }

        Ok(())
    }

    // ─── Capabilities ────────────────────────────────────────────────────────

    fn handle_capabilities(&self) -> Result<()> {
        protocol::write_line("import")?;
        protocol::write_line("push")?;
        protocol::write_line("option")?;
        protocol::write_line(&format!("refspec {}", REFSPEC))?;
        protocol::write_blank()?;
        Ok(())
    }

    // ─── List refs ───────────────────────────────────────────────────────────

    async fn handle_list(&self, _for_push: bool) -> Result<()> {
        let store = self
            .storage
            .load_refs()
            .await
            .context("Failed to load refs for list")?;

        if store.refs.is_empty() {
            protocol::write_blank()?;
            return Ok(());
        }

        let head_target = Self::determine_head(&store);

        for (ref_name, sha) in &store.refs {
            protocol::write_line(&format!("{} {}", sha, ref_name))?;
        }

        if let Some(head) = head_target {
            protocol::write_line(&format!("@{} HEAD", head))?;
        }

        protocol::write_blank()?;
        Ok(())
    }

    fn determine_head(store: &RefsStore) -> Option<String> {
        let preferred = ["refs/heads/main", "refs/heads/master"];
        for p in &preferred {
            if store.refs.contains_key(*p) {
                return Some(p.to_string());
            }
        }
        store.refs.keys().next().cloned()
    }

    // ─── Import (fetch) ──────────────────────────────────────────────────────

    async fn handle_import(&self, refs: &[String]) -> Result<()> {
        info!("import requested for refs: {:?}", refs);

        let store = self.storage.load_refs().await?;

        if store.bundles.is_empty() {
            self.emit_empty_fast_import()?;
            return Ok(());
        }

        let tmp_dir = std::env::temp_dir().join(format!("git-remote-opendal-{}", process_id()));
        std::fs::create_dir_all(&tmp_dir)?;

        for bundle_key in &store.bundles {
            let data = self
                .storage
                .download_bundle(bundle_key)
                .await
                .with_context(|| format!("Failed to download bundle: {}", bundle_key))?;

            let bundle_path = tmp_dir.join("current.bundle");
            std::fs::write(&bundle_path, &data)?;

            let status = SysCommand::new("git")
                .args(["bundle", "unbundle", bundle_path.to_str().unwrap()])
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .status()
                .context("Failed to run 'git bundle unbundle'")?;

            if !status.success() {
                bail!("'git bundle unbundle' failed with status {:?}", status);
            }
        }

        let _ = std::fs::remove_dir_all(&tmp_dir);

        protocol::write_raw(b"feature done\n")?;

        for ref_name in refs {
            if let Some(sha) = store.refs.get(ref_name) {
                protocol::write_raw(format!("reset {}\nfrom {}\n\n", ref_name, sha).as_bytes())?;
            } else {
                warn!("Requested ref '{}' not found in remote store", ref_name);
            }
        }

        protocol::write_raw(b"done\n")?;

        Ok(())
    }

    fn emit_empty_fast_import(&self) -> Result<()> {
        protocol::write_raw(b"feature done\n")?;
        protocol::write_raw(b"done\n")?;
        Ok(())
    }

    // ─── Push ────────────────────────────────────────────────────────────────

    /// Handle a batch of `push <src>:<dst>` commands.
    ///
    /// For each ref being pushed, resolves the local SHA, creates a git bundle
    /// containing only the new commits (excluding anything already on the
    /// remote), uploads it, and updates refs.json.
    async fn handle_push(&mut self, pushes: &[(String, String)]) -> Result<()> {
        info!("push: {} ref(s)", pushes.len());

        let mut store = self.storage.load_refs().await?;
        let mut updated_refs: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for (src, dst) in pushes {
            if src.is_empty() {
                // Deletion: `push :dst` — not yet supported.
                warn!("push: deletion of '{}' not supported", dst);
                protocol::write_line(&format!("error {} deletion not supported", dst))?;
                continue;
            }
            let sha = self.resolve_ref(src)?;
            updated_refs.insert(dst.clone(), sha);
        }

        if updated_refs.is_empty() {
            protocol::write_blank()?;
            return Ok(());
        }

        let bundle_name = format!("{:016x}", timestamp_ms());
        let tmp_bundle = std::env::temp_dir().join(format!(
            "git-remote-opendal-{}-{}.bundle",
            process_id(),
            bundle_name
        ));

        let mut bundle_cmd = SysCommand::new("git");
        bundle_cmd.arg("bundle").arg("create").arg(&tmp_bundle);
        for ref_name in updated_refs.keys() {
            bundle_cmd.arg(ref_name);
        }
        // Exclude commits the remote already has so bundles stay incremental.
        for old_sha in store.refs.values() {
            bundle_cmd.arg(format!("^{}", old_sha));
        }

        let status = bundle_cmd
            .stderr(Stdio::inherit())
            .status()
            .context("Failed to run 'git bundle create'")?;

        if !status.success() {
            bail!("'git bundle create' failed");
        }

        let bundle_data = std::fs::read(&tmp_bundle).context("Failed to read bundle")?;
        let _ = std::fs::remove_file(&tmp_bundle);

        let bundle_key = self
            .storage
            .upload_bundle(&bundle_name, bundle_data)
            .await?;
        store.bundles.push(bundle_key);
        for (dst, sha) in &updated_refs {
            store.refs.insert(dst.clone(), sha.clone());
        }
        self.storage.save_refs(&store).await?;

        for dst in updated_refs.keys() {
            protocol::write_line(&format!("ok {}", dst))?;
        }
        protocol::write_blank()?;

        info!("push: {} ref(s) updated", updated_refs.len());
        Ok(())
    }

    fn resolve_ref(&self, refname: &str) -> Result<String> {
        let output = SysCommand::new("git")
            .args(["rev-parse", refname])
            .output()
            .context("Failed to run git rev-parse")?;

        if !output.status.success() {
            bail!("Failed to resolve ref '{}'", refname);
        }

        Ok(String::from_utf8(output.stdout)
            .context("git rev-parse output is not UTF-8")?
            .trim()
            .to_string())
    }

    // ─── Option ──────────────────────────────────────────────────────────────

    fn handle_option(&self, key: &str, val: &str) -> Result<()> {
        debug!("option: {}={}", key, val);
        match key {
            "verbosity" => protocol::write_line("ok")?,
            "progress" => protocol::write_line("ok")?,
            _ => protocol::write_line("unsupported")?,
        }
        Ok(())
    }
}

// ─── Utilities ───────────────────────────────────────────────────────────────

fn process_id() -> u32 {
    std::process::id()
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
