//! What cargo built for a test program: a library its tests load, or an
//! example, found by cargo's own rule wherever the build directory is.
//!
//! Cargo writes a crate's libraries and examples into the build directory's
//! profile folder, `debug` in a test build, and its test programs below
//! that folder: `deps/` before Cargo 1.100, a unit's own
//! `build/<package>/<hash>/out/` since. The build directory is
//! `CARGO_TARGET_DIR`, else the workspace root's `target`: in the estate,
//! one workspace, its root's (ADR-0016, amendment 2026-11-20); for a
//! repository checked out alone, its own. So what was built is in a folder
//! above the running test program, and nothing here names a build
//! directory: the one rule the estate's tests find built files by.

use std::path::{Path, PathBuf};

/// The file at `relative` in the nearest folder above the running program
/// that holds it: a library, `xmip_core_test_module.dll`, or an example,
/// `examples/<file>`. `None` where no folder above it does.
#[must_use]
pub fn found(relative: impl AsRef<Path>) -> Option<PathBuf> {
    let program = std::env::current_exe().ok()?;
    program
        .ancestors()
        .map(|folder| folder.join(relative.as_ref()))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_program_is_found_above_itself_and_a_name_nothing_holds_is_not() {
        let program = std::env::current_exe().expect("the test program");
        let name = program.file_name().expect("a file name");
        assert_eq!(found(name), Some(program.clone()));
        assert_eq!(found("no-such-file-was-built.xmip"), None);
    }
}
