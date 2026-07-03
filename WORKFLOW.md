# Workflow: Storage-Backed Git Remote Repository

This document defines a practical workflow for creating and operating a Git
remote repository on top of `git-remote-opendal`.

The goal is to make "my own Git remote registry" feel like a normal Git
workflow while keeping storage-backed details explicit and manageable.

## 1. Choose the storage backend

Pick the OpenDAL backend that will hold the repository data.

Supported user-facing backends:

- `s3`
- `gcs`
- `azblob`
- `gdrive`
- `fs`

Use `fs` for local testing, and a cloud backend for real usage.

## 2. Decide the repository location

Define one storage namespace per repository.

Recommended pattern:

```text
opendal://fs/<root-path>
opendal://gdrive/<root-path>
opendal://s3/<bucket>/<root-path>
```

Examples:

- `opendal://fs/tmp/my-bare-repos/myrepo`
- `opendal://s3/my-git-bucket/company-git/prod/myrepo`
- `opendal://gcs/team-storage/frontend`

For `s3`, `gcs`, and `azblob`, the first path segment is the bucket or
container name. The remaining path is the repository root inside that storage
namespace. `fs` and `gdrive` use the path directly as the repository root.

## 3. Provision storage access

Create the bucket, container, or filesystem directory that will store the
repository.

Then configure credentials using environment variables of the form:

```text
OPENDAL_<SCHEME>_<PARAM>=<value>
```

Examples:

```bash
export OPENDAL_S3_BUCKET=my-git-bucket
export OPENDAL_S3_REGION=us-east-1
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
```

```bash
export OPENDAL_AZBLOB_CONTAINER=my-git-container
export OPENDAL_AZBLOB_ACCOUNT_NAME=myaccount
export OPENDAL_AZBLOB_ACCOUNT_KEY=...
```

Keep credentials out of git config and out of repository history.

## 4. Register the remote in Git

Add the storage-backed URL as a normal Git remote.

```bash
git remote add origin opendal://s3/my-git-bucket/company-git/prod/myrepo
```

At this point the repository does not need any special Git configuration.

## 5. Bootstrap the repository with the first push

The first successful push creates the remote repository state in storage.

```bash
git push origin main
```

What happens on the first push:

- Git sends the ref update request.
- `git-remote-opendal` creates a bundle for the pushed history.
- The bundle is uploaded to storage under `objects/`.
- `info/refs.json` is written or updated with:
  - the current ref map
  - the ordered list of bundle objects

This is the moment the storage path becomes a real Git remote.

## 6. Clone and fetch normally

Once initialized, other users can clone from the same remote URL.

```bash
git clone opendal://s3/my-git-bucket/company-git/prod/myrepo
```

Fetch and clone work by:

- reading `info/refs.json`
- downloading each stored bundle in order
- replaying the bundles into Git

The helper reconstructs repository state from storage on demand.

## 7. Push updates as normal Git operations

After the repository is created, day-to-day usage is standard Git.

```bash
git push origin main
git push origin feature/login:feature/login
git fetch origin
```

Each push appends a new bundle and updates the stored ref map.

## 8. Treat the storage prefix as the registry

For operational purposes, the storage prefix is the repository registry.

Use a predictable layout such as:

- one bucket or container per environment
- one prefix per organization
- one path per repository

Example:

```text
s3://company-git/dev/frontend
s3://company-git/dev/backend
s3://company-git/prod/frontend
```

This makes backup, migration, and access control easier to reason about.

## 9. Recommended operating rules

- Use separate credentials for read-only and read-write access.
- Avoid concurrent pushes to the same repository when possible.
- Prefer one repository per storage prefix.
- Use `fs` only for local testing and development.
- Keep the remote URL stable once shared.

## 10. Known behavior to account for

- `info/refs.json` is the canonical metadata file.
- `objects/*.bundle` are append-only bundle files.
- Concurrent pushes are effectively last-writer-wins for `refs.json`.
- Large histories are reconstructed by downloading all recorded bundles.
- Shallow clones are not supported.

## 11. Suggested lifecycle

This is the simplest lifecycle for a storage-backed Git remote:

1. Provision storage.
2. Configure credentials.
3. Add the remote URL.
4. Push the first branch.
5. Share the clone URL.
6. Fetch and push normally.
7. Back up or migrate by copying the storage prefix.

## 12. Future improvements

This tool can be extended with a more guided registry workflow:

- `init` command to create an empty repository layout
- `status` command to inspect remote metadata
- `doctor` command to validate credentials and storage access
- locking or lease support for safer concurrent pushes
- bundle pruning and retention policies
- repo migration helpers for moving between backends

## 13. Practical example

Local testing with the filesystem backend:

```bash
git remote add origin opendal://fs/tmp/my-bare-repos/myrepo
git push origin main
git clone opendal://fs/tmp/my-bare-repos/myrepo local-clone
```

Cloud-backed example:

```bash
export OPENDAL_S3_BUCKET=my-git-bucket
export OPENDAL_S3_REGION=us-east-1
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...

git remote add origin opendal://s3/my-git-bucket/company-git/prod/myrepo
git push origin main
git clone opendal://s3/my-git-bucket/company-git/prod/myrepo
```
