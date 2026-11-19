#![forbid(unsafe_code)]
//! File system acts every layer shares, one copy of each: a directory's
//! names on disk as they are now ([`synced`]).
//!
//! A Library, below every other layer (ADR-0058): Foundation opens no file
//! (ADR-0058 clause 1), so the act sits here, where the file transport and
//! the archive both reach it (ADR-0044, amendment 2026-11-19).

/// A directory's names on disk as they are now, once for every writer that
/// renames or links a file into place.
pub mod synced;
