//! A directory's names on disk as they are now: what makes a file renamed
//! or linked into a directory still there under its name after the machine
//! stops, never the name it had before or none.
//!
//! Every writer that puts a name in place and then answers for it syncs
//! the directory through here — a sent file renamed over its target
//! (`xmip-core-transport-file`), an archived file linked into place
//! (`xmip-core-archive`'s `whole`, which the file and Parquet archives
//! write with). Windows syncs a directory through a handle
//! to it opened for writing with `FILE_FLAG_BACKUP_SEMANTICS`, which
//! `sync_all` flushes (`FlushFileBuffers`); Unix through the directory
//! opened to read (`fsync`). A caller waits on it through its own account
//! of the disk's time, `xcore::spent::on_sync`, never Xmip's own share
//! (ADR-0018, amendment 2026-11-09): a Library depends on no Foundation
//! crate, so the account is the caller's to keep.

use std::fs::File;
use std::io;
use std::path::Path;

/// The directory `path` is in synced: the names in it on disk as they are
/// now. A `path` with no directory part is in the current one.
///
/// # Errors
/// The directory could not be opened, or the sync failed.
pub fn directory(path: &Path) -> io::Result<()> {
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let opened = cfg_select! {
        windows => {{
            use std::os::windows::fs::OpenOptionsExt;
            /// `FILE_FLAG_BACKUP_SEMANTICS`: what opens a directory at all.
            const BACKUP_SEMANTICS: u32 = 0x0200_0000;
            /// `GENERIC_WRITE`: what `FlushFileBuffers` asks of its handle.
            const GENERIC_WRITE: u32 = 0x4000_0000;
            File::options()
                .access_mode(GENERIC_WRITE)
                .custom_flags(BACKUP_SEMANTICS)
                .open(directory)
        }}
        _ => File::open(directory),
    };
    opened?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporary::Folder;

    #[test]
    fn the_directory_a_file_is_in_is_synced() {
        let folder = Folder::new("synced");
        directory(&folder.join("order.edi")).expect("the directory synced");
        let missing = directory(&folder.join("gone").join("order.edi"));
        assert!(missing.is_err(), "a directory not there is not synced");
    }
}
