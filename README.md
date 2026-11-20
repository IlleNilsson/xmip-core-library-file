# xmip-core-library-file

File system acts every layer shares, one copy of each. A Library: below
every other layer, depending on nothing of the estate's (ADR-0058).

- **A directory synced, `synced::directory`.** The names in a directory on
  disk as they are now, so a file renamed or linked into it outlives a
  crash under its new name, never the one it had before or none. Unix
  syncs the directory opened to read (`fsync`); Windows flushes a handle to
  it opened for writing with `FILE_FLAG_BACKUP_SEMANTICS`
  (`FlushFileBuffers`). Called by the file transport's send and by the
  archive's `whole`, which the file and Parquet archives write with; each
  caller waits on it through `xcore::spent::on_sync`, the disk's time and
  never Xmip's own share (ADR-0018, amendment 2026-11-09).
- **A file held by its writer, `held::create`, and one no process holds
  removed, `held::remove_unheld`.** A file written under a name of its own
  before it is linked to its name is held by its process for as long as it
  is open, from the moment it is made: on Unix by an exclusive lock
  (`flock`), the file made again where a removal came between its making
  and its lock; on Windows by the sharing it is opened with, which lets
  another handle read it and never write, rename or remove it. A process
  that dies has closed what it held, so a removal takes only a file no live
  process writes, and reads no name, process id or age to decide it. The
  archive's `whole` writes with it and removes what a dead process left
  (ADR-0040, amendment 2026-11-20).
- **A test's own folder, `temporary::Folder`, under `test-support`.**
  `xmip-<label>-<UUIDv7>` under the system's temporary directory, the
  UUIDv7 the `uuid` crate's, its random bits the operating system's; made
  with `create_dir`, so a folder already there is refused rather than taken
  over; on Unix its owner's alone; and removed with everything in it when
  dropped, also when the test fails. The one way the estate's Rust tests
  make a folder, taken from their dev-dependencies and never in a node
  (ADR-0021, amendment 2026-11-19; `doc/governance/rust-style.md` section
  5f).

Here since 2026-11-19: written once in the file transport, moved to
`xmip-core` for the archive the same day, and moved here because a
Foundation crate opens no file (ADR-0058 clause 1; ADR-0044, amendment
2026-11-19).

## Verification

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Licensed under the AGPL-3.0-or-later; see `LICENSE`.
