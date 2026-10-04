//! Replace a file in one step, or leave it as it was.
//!
//! Writing straight into the destination truncates it first and refills it
//! after. Anything that interrupts the refill — a full disk, a crash, a power
//! cut, an encoder error — leaves the file short, and for a snip that file is
//! the only copy. Everything here writes a sibling temporary file, forces it to
//! disk, and only then renames it over the destination: a reader, or the next
//! launch, sees the old file or the new one, never half of either. A sibling,
//! because a rename is only atomic within one filesystem.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Context;

/// Marks every temporary file this module creates, so a sweep can tell ours
/// from anything else in the directory.
const TEMP_SUFFIX: &str = ".snipdeck-tmp";

/// A fully written temporary file, not yet in place. Dropping it without
/// [`TempFile::commit`] deletes it.
pub struct TempFile {
    tmp: PathBuf,
    dest: PathBuf,
    committed: bool,
}

impl TempFile {
    /// Rename it over the destination.
    pub fn commit(mut self) -> anyhow::Result<()> {
        std::fs::rename(&self.tmp, &self.dest)
            .with_context(|| format!("replacing {}", self.dest.display()))?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.tmp);
        }
    }
}

/// Write `dest`'s replacement into a temporary sibling and sync it to disk.
///
/// `write` gets a buffered writer; whatever it leaves unflushed is flushed
/// here, and a flush failure — the usual way a full disk shows up — is an
/// error rather than a silently short file.
pub fn write_temp(
    dest: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> anyhow::Result<()>,
) -> anyhow::Result<TempFile> {
    let dir = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let temp = TempFile {
        tmp: dir.join(temp_name(dest)),
        dest: dest.to_path_buf(),
        committed: false,
    };
    let file = File::create(&temp.tmp)
        .with_context(|| format!("creating {}", temp.tmp.display()))?;
    let mut writer = BufWriter::new(file);
    write(&mut writer)?;
    writer.flush()?;
    let file = writer.into_inner().map_err(|e| e.into_error())?;
    file.sync_all()?;
    Ok(temp)
}

/// [`write_temp`] and [`TempFile::commit`] in one go.
pub fn replace(
    dest: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    write_temp(dest, write)?.commit()
}

/// Delete temporaries a crash left in `dir`.
///
/// Only call this when nothing can be writing there — at start-up, before the
/// first write, which the single-instance guard makes safe.
pub fn sweep(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(TEMP_SUFFIX) {
            log::info!("removing a temporary file left by a crash: {}", entry.path().display());
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `.<name>.<pid>-<n>.snipdeck-tmp`: hidden, unique within the process and
/// across processes, and recognisable to [`sweep`].
fn temp_name(dest: &Path) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".into());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(".{name}.{}-{n}{TEMP_SUFFIX}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("snipdeck-atomic-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replace_swaps_in_the_new_contents() {
        let dir = scratch_dir("swap");
        let dest = dir.join("a.txt");
        std::fs::write(&dest, b"old").unwrap();
        replace(&dest, |w| Ok(w.write_all(b"new")?)).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no temporary left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The whole point: a writer that fails part-way must not touch the
    /// original, and must not leave its half-written temporary either.
    #[test]
    fn a_failed_write_leaves_the_original_untouched() {
        let dir = scratch_dir("fail");
        let dest = dir.join("a.png");
        std::fs::write(&dest, b"original").unwrap();
        let result = replace(&dest, |w| {
            w.write_all(b"half")?;
            anyhow::bail!("encoder gave up")
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "temporary cleaned up");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sweep_removes_only_our_temporaries() {
        let dir = scratch_dir("sweep");
        std::fs::write(dir.join("keep.png"), b"x").unwrap();
        std::fs::write(dir.join(temp_name(&dir.join("keep.png"))), b"x").unwrap();
        sweep(&dir);
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["keep.png".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
