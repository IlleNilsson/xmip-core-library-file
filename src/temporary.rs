//! A folder of a test's own under the system's temporary directory: the one
//! way the estate's Rust tests make one (ADR-0021, amendment 2026-11-19).
//! Compiled for tests only, with `test-support`.
//!
//! Its name ends in a UUIDv7, whose random bits the operating system gives
//! (the `uuid` crate's `v7`, through `getrandom`), so no other account knows
//! it before it exists. It is made with `create_dir`, which refuses a folder
//! already there rather than taking it over, and on Unix only its owner may
//! enter it. Dropping it removes it with everything in it, also when the test
//! fails: a panicking test drops what it holds as it unwinds.
//!
//! A test makes its folder first, so that it is dropped last, after every
//! file, database or socket the test opened in it: Windows removes no file
//! that is still open.

use std::fs;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// A folder of a test's own, removed with everything in it when dropped.
#[derive(Debug)]
pub struct Folder {
    path: PathBuf,
}

impl Folder {
    /// Makes `xmip-<label>-<UUIDv7>` under the system's temporary directory.
    ///
    /// # Panics
    /// The folder cannot be made.
    #[must_use]
    pub fn new(label: &str) -> Self {
        let name = format!("xmip-{label}-{}", uuid::Uuid::now_v7().simple());
        let path = std::env::temp_dir().join(name);
        made(&path).unwrap_or_else(|error| panic!("cannot make {}: {error}", path.display()));
        Self { path }
    }
}

/// `path` made as a folder of its own, refused where anything is already.
fn made(path: &Path) -> io::Result<()> {
    cfg_select! {
        unix => {
            use std::os::unix::fs::DirBuilderExt as _;
            fs::DirBuilder::new().mode(0o700).create(path)
        }
        _ => fs::create_dir(path),
    }
}

impl Deref for Folder {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Folder {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Where it is, for what takes a path of its own: the folder stays the
/// test's, and is removed when the test drops it.
impl From<&Folder> for PathBuf {
    fn from(folder: &Folder) -> Self {
        folder.path.clone()
    }
}

impl Drop for Folder {
    /// Never panics: a panic while a failing test unwinds would end the
    /// whole run. A folder that cannot be removed is named on standard error.
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!("a test's folder {} is left: {error}", self.path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn a_folder_is_made_under_the_temporary_directory_and_removed_with_its_content() {
        let folder = Folder::new("temporary-test");
        let path = folder.to_path_buf();
        assert!(path.is_dir());
        assert_eq!(path.parent(), Some(std::env::temp_dir().as_path()));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        assert!(name.starts_with("xmip-temporary-test-"), "{name}");
        fs::create_dir(folder.join("inner")).expect("an inner folder");
        fs::write(folder.join("inner").join("order.edi"), b"UNA").expect("a file");
        drop(folder);
        assert!(!path.exists(), "{} is left", path.display());
    }

    #[test]
    fn two_folders_of_one_label_are_two() {
        let (first, second) = (Folder::new("temporary-test"), Folder::new("temporary-test"));
        assert_ne!(first.to_path_buf(), second.to_path_buf());
    }

    #[test]
    fn a_folder_already_there_is_refused_not_taken_over() {
        let folder = Folder::new("temporary-test");
        let refused = made(&folder).expect_err("already there");
        assert_eq!(refused.kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn a_folder_is_removed_also_when_the_test_panics() {
        let made_at = Mutex::new(None);
        let unwound = std::panic::catch_unwind(|| {
            let folder = Folder::new("temporary-test");
            *made_at
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(folder.to_path_buf());
            panic!("the test fails");
        });
        assert!(unwound.is_err());
        let path = made_at
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .expect("made before the panic");
        assert!(!path.exists(), "{} is left", path.display());
    }

    #[cfg(unix)]
    #[test]
    fn on_unix_only_its_owner_may_enter_it() {
        use std::os::unix::fs::PermissionsExt as _;
        let folder = Folder::new("temporary-test");
        let mode = fs::metadata(&folder)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "{mode:o}");
    }
}
