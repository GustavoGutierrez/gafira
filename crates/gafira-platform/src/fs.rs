//! `FileSystem` over `std::fs` (02 §7.2): the rename of the atomic save and
//! the sync of its folder.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use gafira_ports::system::{FileSystem, FileWrite, ReadSeek};

/// The real file system.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdFileSystem;

struct Written(File);

impl io::Write for Written {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl io::Seek for Written {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}

impl FileWrite for Written {
    fn sync_all(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }
}

impl FileSystem for StdFileSystem {
    fn open(&self, path: &Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        Ok(Box::new(File::open(path)?))
    }

    fn create(&self, path: &Path) -> io::Result<Box<dyn FileWrite>> {
        Ok(Box::new(Written(File::create(path)?)))
    }

    /// `rename` replaces the target atomically on Linux and macOS, and on
    /// Windows through `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`; then the
    /// folder is synced where the OS allows it (Unix), so the new name
    /// survives a power cut.
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)?;
        if cfg!(unix)
            && let Some(folder) = to.parent().filter(|p| !p.as_os_str().is_empty())
        {
            File::open(folder)?.sync_all()?;
        }
        Ok(())
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        path.try_exists()
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        fs::read_dir(path)?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gafira-platform-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        dir
    }

    #[test]
    fn s18_01_write_sync_replace_and_read_back() {
        let dir = scratch("fs");
        let fs = StdFileSystem;
        let target = dir.join("poster.gaf");
        fs::write(&target, b"old").expect("the old file");
        let tmp = dir.join("poster.gaf.tmp-0");
        let mut file = fs.create(&tmp).expect("create");
        file.write_all(b"new").expect("write");
        file.sync_all().expect("sync");
        drop(file);
        fs.replace(&tmp, &target).expect("replace");

        let mut text = String::new();
        fs.open(&target)
            .expect("open")
            .read_to_string(&mut text)
            .expect("read");
        assert_eq!(text, "new");
        assert!(!fs.exists(&tmp).expect("lookup"));
        assert_eq!(fs.read_dir(&dir).expect("list"), vec![target.clone()]);
        fs.remove(&target).expect("remove");
        assert!(!fs.exists(&target).expect("lookup"));
        fs::remove_dir_all(dir).expect("clean up");
    }

    #[test]
    fn s18_01_errors_come_back_as_io_errors() {
        let dir = scratch("fs-errors");
        let fs = StdFileSystem;
        let missing = dir.join("missing.gaf");
        assert_eq!(
            fs.open(&missing).err().map(|e| e.kind()),
            Some(io::ErrorKind::NotFound)
        );
        assert!(fs.replace(&missing, &dir.join("x")).is_err());
        fs::remove_dir_all(dir).expect("clean up");
    }
}
