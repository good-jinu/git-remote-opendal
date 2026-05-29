## git-remote-opendal

This project is a Git remote helper that stores repositories on any OpenDAL
backend (S3, GCS, Azure Blob, local fs, etc.). The goal of this file is to
capture immediately useful, discoverable facts for an AI so it can make
productive edits without guessing project conventions.

- Language: Rust (async, tokio). Binary invoked by Git as `git-remote-opendal`.
- Entry point: `src/main.rs` (parses args, builds operator, runs helper state machine).

Core components (read across these files to understand flows):

- `src/main.rs` — CLI entry, tracing config (logs go to stderr), builds the
  OpenDAL operator, starts `helper::RemoteHelper`.
- `src/config.rs` — URL parsing and parameter resolution. Priority: git config
  (remote.<name>.opendal<Scheme><Param>) > environment variables
  (`OPENDAL_<SCHEME>_<KEY>`). Keys used inside code are lowercase-hyphenated
  (e.g. `access-key-id`, `credential-path`). See `collect_git_params` and
  `collect_env_params` for exact behavior.
- `src/operator.rs` — constructs `opendal::Operator` per scheme. Supported
  schemes: `s3`, `gcs`, `azblob`, `gdrive`, `fs`, `memory`. To add a backend:
  1) enable the service feature in `Cargo.toml`; 2) add `build_<scheme>`;
  3) add an arm in `build_operator`.
- `src/storage.rs` — storage layout and helpers. Important layout:

  info/refs.json  — JSON { refs: {"refs/heads/...": "<sha>"}, bundles: ["objects/....bundle"] }
  objects/*.bundle — one bundle file per push

  RefsStore has fields `refs: HashMap<String,String>` and `bundles: Vec<String>`.

- `src/protocol.rs` — small parser/writer for Git remote helper line protocol.
  Use `write_raw` for fast-import data and `write_line` / `write_blank` for
  protocol lines. Note: diagnostics MUST go to stderr (see tracing setup).
- `src/helper.rs` — main protocol state machine implementing `import`/`export`.
  Push flow uses `git fast-import`/`git bundle create` and uploads bundles.
  Fetch flow downloads bundles and runs `git bundle unbundle` then emits a
  fast-import stream. Both flows rely on the `git` binary being present.

Developer workflows (commands discovered from README / files):

- Build (dev):
  cargo install --path .
  # or for local debugging
  cargo build --release

- Tests: run unit tests (module tests exist in several files):
  cargo test

- Debug logs: set `GIT_REMOTE_OPENDAL_LOG` before invoking git to control
  tracing (goes to stderr):
  export GIT_REMOTE_OPENDAL_LOG=debug

- Manual smoke (local fs):
  git remote add origin opendal://fs/tmp/my-bare-repos/myrepo.git
  git push origin main
  git clone opendal://fs/tmp/my-bare-repos/myrepo.git

Project-specific conventions and gotchas:

- Config precedence: git config entries under `remote.<name>.opendal<Scheme><Param>` win
  over `OPENDAL_<SCHEME>_<KEY>` env vars. `config.rs` implements this via
  `git config --get-regexp` — changes to that code affect precedence.
- Param naming: git keys are camelCase suffixes in `.git/config` but are
  mapped to lowercase-hyphenated keys internally. See `normalize_git_param_key`.
- Logging: tracing is configured to write to stderr intentionally. Do not
  write protocol data to stderr — it will break Git's protocol (see main.rs).
- Storage layout: `refs.json` contains the canonical list of refs and the
  ordered list of bundle keys. Concurrent pushes use a last-writer-wins
  strategy — there's no server-side locking implemented.
- Temporary files: imports/exports use the system temp dir and `git bundle`
  tooling. The helper assumes `git` is available on PATH and that subprocess
  calls to `git` succeed; update `helper.rs` if you need platform-specific
  adjustments.

Extensibility notes (common edits you may be asked to make):

- Add a new backend: modify `Cargo.toml` to add opendal feature, then add
  a `build_<name>` in `src/operator.rs` and an arm in `build_operator`.
- Change ref storage shape: `src/storage.rs::RefsStore` centralises the
  format—update read/write logic and consider migration when changing it.
- Protocol changes: `src/protocol.rs` is the single place for parsing and
  writing lines. Modifying it affects `helper.rs` heavily.

Where to look first for a change request
- If it's about config/env handling: `src/config.rs`.
- If it's about cloud client setup: `src/operator.rs` and Cargo features.
- If it's about wire protocol or helper behavior: `src/protocol.rs` and
  `src/helper.rs`.

If anything in this file is unclear or you'd like more examples (e.g., a
step-by-step local debug session or an integration test harness), tell me
which part to expand and I'll update this doc.
