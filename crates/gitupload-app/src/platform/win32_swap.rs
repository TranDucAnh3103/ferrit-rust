#![forbid(unsafe_code)]
//! Atomic Windows executable hot-swap renaming running binaries on NTFS before replacement.

use std::io;
use std::path::Path;

/// Atomically swaps the currently running binary with `new_binary_path` on NTFS (`current -> .old -> new`).
pub fn atomic_executable_swap(new_binary_path: &Path) -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    atomic_swap_at(&current_exe, new_binary_path)
}

/// Core NTFS atomic replacement routine separating file target resolution for deterministic testing.
pub fn atomic_swap_at(current_exe: &Path, new_binary_path: &Path) -> io::Result<()> {
    if !new_binary_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Staged update binary does not exist: {}",
                new_binary_path.display()
            ),
        ));
    }

    let backup_exe = current_exe.with_extension("exe.old");
    if backup_exe.exists() {
        let _ = std::fs::remove_file(&backup_exe);
    }

    // Windows NTFS allows renaming a running locked binary out of the way cleanly.
    std::fs::rename(current_exe, &backup_exe)?;
    if let Err(err) = std::fs::copy(new_binary_path, current_exe) {
        // Attempt rollback if copy failed
        let _ = std::fs::rename(&backup_exe, current_exe);
        return Err(err);
    }

    let _ = std::fs::remove_file(&backup_exe);
    Ok(())
}
