# git-remote-opendal

Use any cloud storage — **S3, GCS, Azure Blob, local filesystem, and [50+ more](https://opendal.apache.org)** — as a plain Git remote.

Backed by [Apache OpenDAL™](https://opendal.apache.org), one unified API for all storage.

---

## How it works

`git-remote-opendal` is a [Git remote helper](https://git-scm.com/docs/gitremote-helpers).
Git invokes it automatically when it sees a remote URL starting with `opendal://`.

The helper implements the **`import`/`export`** capability pair:

- **Push** (`git push`): git streams a `git fast-export` bundle → helper converts
  it to a `.bundle` file → uploads to cloud storage → updates `info/refs.json`.
- **Fetch/Clone** (`git fetch`, `git clone`): helper downloads all stored bundles
  in order → pipes them through `git bundle unbundle` → feeds the resulting
  fast-import stream back to git.

### Storage layout (inside the backend root)

```
<root>/
  info/
    refs.json        ← ref name → SHA-1 map + ordered bundle list
  objects/
    <timestamp>.bundle  ← one bundle per push
```

---

## Requirements

- Rust ≥ 1.85 (OpenDAL 0.56 MSRV)
- Git ≥ 2.20

---

## Installation

```bash
cargo install --path .
# Binary must be on PATH as 'git-remote-opendal'
```

Or copy the built binary to somewhere on your `$PATH`:

```bash
cargo build --release
cp target/release/git-remote-opendal ~/.local/bin/
```

---

## Usage

### URL format

```
opendal://<scheme>/<root-path>
```

| Part | Description |
|------|-------------|
| `scheme` | OpenDAL backend: `s3`, `gcs`, `azblob`, `fs`, `memory` |
| `root-path` | Path inside the backend that acts as the repository root |

### Backend configuration

Backend parameters come from environment variables using the pattern:

```
OPENDAL_<SCHEME>_<PARAM>=<value>
```

This keeps credentials out of git-config and out of repository history.

---

## Examples

### Amazon S3 / S3-compatible (MinIO, Cloudflare R2, …)

```bash
# Configure the remote
git remote add origin opendal://s3/my-git-repos/myrepo.git

# Set credentials (never in git-config!)
export OPENDAL_S3_BUCKET=my-git-bucket
export OPENDAL_S3_REGION=us-east-1
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=...

# Use normally
git push origin main
git clone opendal://s3/my-git-repos/myrepo.git
```

For S3-compatible endpoints (MinIO, R2, etc.):

```bash
export OPENDAL_S3_ENDPOINT=https://my-minio.example.com
export OPENDAL_S3_REGION=us-east-1   # required even for MinIO
```

---

### Google Cloud Storage

```bash
git remote add origin opendal://gcs/my-git-repos/myrepo.git

export OPENDAL_GCS_BUCKET=my-gcs-bucket
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
# or: export OPENDAL_GCS_CREDENTIAL=$(base64 service-account.json)

git push origin main
```

---

### Azure Blob Storage

```bash
git remote add origin opendal://azblob/my-git-repos/myrepo.git

export OPENDAL_AZBLOB_CONTAINER=my-git-container
export OPENDAL_AZBLOB_ACCOUNT_NAME=myaccount
export OPENDAL_AZBLOB_ACCOUNT_KEY=...

git push origin main
```

---

### Local filesystem (testing)

```bash
git remote add origin opendal://fs/tmp/my-bare-repos/myrepo.git

git push origin main
git clone opendal://fs/tmp/my-bare-repos/myrepo.git local-clone
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build locally, run tests,
add a new backend, and the project conventions.

---

## Limitations

- **Concurrent pushes**: last-writer-wins for `refs.json`.  Use a separate
  coordination mechanism (e.g. object locking, DynamoDB) for teams.
- **Large histories**: all bundles are downloaded on fetch.  Incremental
  fetch (only new bundles since last fetch) is on the roadmap.
- **Shallow clones** (`--depth`): not supported.

---

## License

MIT