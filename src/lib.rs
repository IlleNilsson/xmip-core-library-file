#![forbid(unsafe_code)]
//! File system acts every layer shares, one copy of each: a directory's
//! names on disk as they are now ([`synced`]), and a file held by its
//! writer while it is open, removed only where no process holds it
//! ([`held`]).
//!
//! A Library, below every other layer (ADR-0058): Foundation opens no file
//! (ADR-0058 clause 1), so the act sits here, where the file transport and
//! the archive both reach it (ADR-0044, amendments 2026-11-19 and
//! 2026-11-20).

/// A file in its writer's hands while it is written, and a file no process
/// holds removed: what a writer that links a file into place writes with.
pub mod held;
/// A directory's names on disk as they are now, once for every writer that
/// renames or links a file into place.
pub mod synced;
/// A folder of a test's own, removed when dropped: the one way a test makes
/// one, under `test-support`.
#[cfg(any(test, feature = "test-support"))]
pub mod temporary;
