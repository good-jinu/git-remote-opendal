# Contributing to git-remote-opendal

Thank you for your interest in contributing!  This document covers how to build
and debug the project locally, how to add new storage backends, and the project
conventions you should be aware of before sending a patch.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building](#building)
- [Running Tests](#running-tests)
- [Debugging](#debugging)
- [Adding a New Backend](#adding-a-new-backend)
- [Project Conventions](#project-conventions)
- [Code Map](#code-map)

---

## Prerequisites

- Rust ≥ 1.85 (OpenDAL 0.56 MSRV)
- Git ≥ 2.20

---

## Building

```bash
# Install directly into your PATH for end-to-end smoke tests
cargo install --path .

# Or build a release binary without installing
cargo build --release
```

---

## Running Tests

```bash
cargo test
```

Unit tests live as `#[cfg(test)]` modules inside each source file.

### Local smoke test (filesystem backend)

```bash
# Terminal 1 – set up a repo and push
git remote add origin opendal://fs/tmp/my-bare-repos/myrepo.git
git push origin main

# Terminal 2 – clone from the same path
git clone opendal://fs/tmp/my-bare-repos/myrepo.git local-clone
```

---

## Debugging

Set `GIT_REMOTE_OPENDAL_LOG` before running any `git` command to control the
[`tracing`](https://docs.rs/tracing) output level:

```bash
export GIT_REMOTE_OPENDAL_LOG=debug
git push origin main
```

All log output goes to **stderr** intentionally — writing anything to stdout
would corrupt Git's wire protocol.

---

## Adding a New Backend

OpenDAL supports 50+ services.  Follow these three steps to add one:

### 1. Enable the feature flag in `Cargo.toml`

```toml
opendal = { version = "0.56", features = ["services-webdav"] }
```

### 2. Add a builder function in [`src/operator.rs`](src/operator.rs)

```rust
fn build_webdav(cfg: &RemoteConfig) -> Result<Operator> {
    use opendal::services::Webdav;
    let mut b = Webdav::default();
    b.endpoint(cfg.params.get("endpoint").unwrap_or_default());
    b.root(&cfg.root);
    Operator::new(b)?.finish().pipe_ok()
}
```

### 3. Add an arm in `build_operator`

```rust
"webdav" => build_webdav(cfg)?,
```

---

## Project Conventions

### Config / parameter resolution

Parameters are resolved via **Environment variables** — `OPENDAL_<SCHEME>_<KEY>`.

Keys are stored internally as lowercase-hyphenated strings (e.g. `access-key-id`).

### Logging

- Use `tracing` macros (`debug!`, `info!`, `warn!`, `error!`).
- **Never** write protocol data to stderr; that breaks Git's wire protocol.

### Storage layout

```
<root>/
  info/
    refs.json        ← ref name → SHA-1 map + ordered bundle list
  objects/
    <timestamp>.bundle  ← one bundle per push
```

`RefsStore` in [`src/storage.rs`](src/storage.rs) owns the serialisation logic.
Update read/write there if you change the format, and think about migration.

### Concurrency

Concurrent pushes use a **last-writer-wins** strategy on `refs.json`.  There is
no server-side locking.  If you add locking, do it inside `src/storage.rs`.

---

## Code Map

| File | Purpose |
|------|---------|
| [`src/main.rs`](src/main.rs) | CLI entry point, tracing setup, operator construction |
| [`src/config.rs`](src/config.rs) | URL parsing and parameter resolution |
| [`src/operator.rs`](src/operator.rs) | Builds `opendal::Operator` per scheme |
| [`src/storage.rs`](src/storage.rs) | Storage layout helpers and `RefsStore` |
| [`src/protocol.rs`](src/protocol.rs) | Git remote helper line protocol parser/writer |
| [`src/helper.rs`](src/helper.rs) | `import`/`export` state machine |

---

## Where to Start for Common Changes

| Change type | Files to touch |
|-------------|---------------|
| Config / env handling | `src/config.rs` |
| Cloud client setup | `src/operator.rs`, `Cargo.toml` |
| Wire protocol or helper behaviour | `src/protocol.rs`, `src/helper.rs` |
| Ref storage format | `src/storage.rs` |
