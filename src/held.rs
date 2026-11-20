//! A file in its writer's hands while it is written, and a file no process
//! holds removed.
//!
//! A writer that puts a file in place in two steps — written under a name
//! of its own, then linked to its name — leaves the written file behind
//! where its process dies between the two (`xmip-core-archive`'s `whole`,
//! which the file and Parquet archives write with; ADR-0040, amendment
//! 2026-11-20). Whoever removes such a file must never take one a live
//! process is still writing, and the operating system is what knows which
//! process lives: a file made by [`create`] is held for as long as it is
//! open, a process that dies has closed everything it held, and
//! [`remove_unheld`] removes a file only where no process holds it. No
//! name, process id or age is read to decide it.
//!
//! Unix holds the file by an exclusive lock on it (`flock`, the standard
//! library's `File::lock`), taken as soon as it is made. A removal locks it
//! first and removes only what it could lock, so a removal can come only
//! between the making and the lock; the writer then finds its name gone
//! once it holds the lock, and makes the file again. A file system that
//! takes no lock at all holds none, and a removal there removes nothing.
//! Windows holds the file by the sharing it is opened with, from the moment
//! it is made: another handle may read it, never write, rename or remove
//! it, and a removal refused for that (`ERROR_SHARING_VIOLATION`) is a file
//! held. Windows takes no lock on it: a lock there is mandatory, and would
//! refuse a reader of the name the file is linked to.

use std::fs::{self, File};
use std::io;
use std::path::Path;

/// `path` made new, read and written, and held by this process until the
/// file is closed: no other process removes it meanwhile.
///
/// # Errors
/// `path` is taken, its directory is not there, or the file system refused
/// the file or the lock on it.
pub fn create(path: &Path) -> io::Result<File> {
    create_by(path, &mut |_| {})
}

/// `path` removed where no process holds it: `true` where it was removed,
/// `false` where a process holds it or it is gone already.
///
/// # Errors
/// The file system refused to open or remove it.
pub fn remove_unheld(path: &Path) -> io::Result<bool> {
    let removed = cfg_select! {
        windows => {{
            /// `ERROR_SHARING_VIOLATION`: a handle open on the file refuses
            /// its removal.
            const SHARING_VIOLATION: i32 = 32;
            match fs::remove_file(path) {
                Err(held) if held.raw_os_error() == Some(SHARING_VIOLATION) => return Ok(false),
                removed => removed,
            }
        }}
        _ => {{
            let file = match File::open(path) {
                Err(gone) if gone.kind() == io::ErrorKind::NotFound => return Ok(false),
                opened => opened?,
            };
            match file.try_lock() {
                Ok(()) => {}
                Err(fs::TryLockError::WouldBlock) => return Ok(false),
                Err(fs::TryLockError::Error(lockless))
                    if lockless.kind() == io::ErrorKind::Unsupported =>
                {
                    return Ok(false);
                }
                Err(fs::TryLockError::Error(cause)) => return Err(cause),
            }
            // Removed while it is locked: closing `file` frees it after.
            fs::remove_file(path)
        }}
    };
    match removed {
        Ok(()) => Ok(true),
        Err(gone) if gone.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(cause) => Err(cause),
    }
}

/// [`create`], calling `made` with the path each time the file is made and
/// before it is held: a test removes it there.
fn create_by(path: &Path, made: &mut dyn FnMut(&Path)) -> io::Result<File> {
    cfg_select! {
        windows => {{
            use std::os::windows::fs::OpenOptionsExt;
            /// `FILE_SHARE_READ`: another handle may read the file, and
            /// nothing more.
            const SHARE_READ: u32 = 0x1;
            let file = File::options()
                .read(true)
                .write(true)
                .create_new(true)
                .share_mode(SHARE_READ)
                .open(path)?;
            made(path);
            Ok(file)
        }}
        _ => loop {
            let file = File::create_new(path)?;
            made(path);
            match file.lock() {
                Ok(()) => {}
                Err(lockless) if lockless.kind() == io::ErrorKind::Unsupported => return Ok(file),
                Err(cause) => return Err(cause),
            }
            if is_at(&file, path)? {
                return Ok(file);
            }
            // Removed between its making and its lock: made again, under
            // the same name, which only this process gives.
        },
    }
}

/// Whether `file` is the file at `path`: on Unix the same device and
/// inode; elsewhere a file at `path` at all.
#[cfg(not(windows))]
fn is_at(file: &File, path: &Path) -> io::Result<bool> {
    let named = match fs::symlink_metadata(path) {
        Err(gone) if gone.kind() == io::ErrorKind::NotFound => return Ok(false),
        named => named?,
    };
    let held = file.metadata()?;
    Ok(cfg_select! {
        unix => {{
            use std::os::unix::fs::MetadataExt;
            named.dev() == held.dev() && named.ino() == held.ino()
        }}
        _ => named.is_file() && held.is_file(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporary::Folder;
    use std::io::{Read, Write};

    #[test]
    fn a_file_held_by_its_writer_is_never_removed_and_once_closed_it_is() {
        let folder = Folder::new("held-writer");
        let path = folder.join("order.xmip-archiving.1.0");
        let mut file = create(&path).expect("made and held");
        file.write_all(b"half").expect("written");
        assert!(!remove_unheld(&path).expect("asked"), "held by its writer");
        let mut read = Vec::new();
        File::open(&path)
            .and_then(|mut other| other.read_to_end(&mut read))
            .expect("another handle reads it");
        assert_eq!(read, b"half");
        assert!(create(&path).is_err(), "taken");
        drop(file);
        assert!(
            remove_unheld(&path).expect("asked"),
            "closed, it is removed"
        );
        assert!(!path.exists());
        assert!(!remove_unheld(&path).expect("asked"), "gone already");
    }

    #[test]
    fn a_removal_in_the_instant_after_the_file_is_made_never_takes_it_from_its_writer() {
        let folder = Folder::new("held-instant");
        let path = folder.join("order.xmip-archiving.1.0");
        let mut removals = Vec::new();
        let file = create_by(&path, &mut |made| {
            if removals.is_empty() {
                removals.push(remove_unheld(made).expect("asked"));
            }
        })
        .expect("made and held");
        // Unix removes it there, before its lock, and it is made again;
        // Windows refuses the removal from the start.
        assert_eq!(removals, [cfg!(unix)]);
        assert!(path.exists(), "at its name");
        assert!(!remove_unheld(&path).expect("asked"), "held by its writer");
        drop(file);
        assert!(remove_unheld(&path).expect("asked"));
    }

    #[test]
    fn a_file_linked_to_its_name_while_held_reads_under_it() {
        let folder = Folder::new("held-linked");
        let path = folder.join("order.xmip-archiving.1.0");
        let mut file = create(&path).expect("made and held");
        file.write_all(b"whole").expect("written");
        fs::hard_link(&path, folder.join("order")).expect("linked while held");
        assert_eq!(fs::read(folder.join("order")).expect("read"), b"whole");
        drop(file);
        assert!(remove_unheld(&path).expect("asked"));
        assert_eq!(fs::read(folder.join("order")).expect("kept"), b"whole");
    }
}
