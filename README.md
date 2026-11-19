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

Here since 2026-11-19: written once in the file transport, moved to
`xmip-core` for the archive the same day, and moved here because a
Foundation crate opens no file (ADR-0058 clause 1; ADR-0044, amendment
2026-11-19).

## Verification

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Licensed under the AGPL-3.0-or-later; see `LICENSE`.
