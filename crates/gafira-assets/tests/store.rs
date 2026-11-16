//! Spec 026: the content-addressed store on disk.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use gafira_assets::FsStore;
use gafira_doc::{AssetRef, ContentHash};
use gafira_ports::store::{ContentStore, StoreError};

/// A store root under the system temporary folder, removed on drop.
struct Root(PathBuf);

impl Root {
    fn new(case: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("gafira-assets-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found
}

fn read_all(store: &FsStore, hash: ContentHash) -> Vec<u8> {
    let mut bytes = Vec::new();
    store
        .open(hash)
        .expect("opens")
        .read_to_end(&mut bytes)
        .expect("reads");
    bytes
}

/// More than one read buffer, so `put` streams.
fn payload() -> Vec<u8> {
    (0..300_000_u32).map(|i| (i % 251) as u8).collect()
}

#[test]
fn s26_01_put_returns_the_blake3_and_stores_under_its_prefix_folder() {
    let root = Root::new("put");
    let store = FsStore::new(&root.0);
    let bytes = payload();
    let hash = store.put(&mut bytes.as_slice()).expect("puts");
    assert_eq!(hash, ContentHash(*blake3::hash(&bytes).as_bytes()));
    let hex = AssetRef::new(hash).hex();
    let path = root.0.join(&hex[..2]).join(&hex);
    assert_eq!(std::fs::read(&path).expect("the stored file"), bytes);
    assert_eq!(files_under(&root.0), vec![path]);
}

#[test]
fn s26_02_open_and_contains_find_what_was_put() {
    let root = Root::new("open");
    let store = FsStore::new(&root.0);
    let hash = store.put(&mut &b"hello"[..]).expect("puts");
    assert!(store.contains(hash).expect("queries"));
    assert_eq!(read_all(&store, hash), b"hello");

    let unknown = ContentHash([7; 32]);
    assert!(!store.contains(unknown).expect("queries"));
    assert!(matches!(
        store.open(unknown),
        Err(StoreError::NotFound { hash }) if hash == unknown
    ));
}

#[test]
fn s26_03_the_same_content_is_stored_once() {
    let root = Root::new("dedup");
    let store = FsStore::new(&root.0);
    let first = store.put(&mut payload().as_slice()).expect("puts");
    let second = store.put(&mut payload().as_slice()).expect("puts again");
    assert_eq!(first, second);
    assert_eq!(files_under(&root.0).len(), 1);
    assert_eq!(read_all(&store, first), payload());
}

/// Yields some bytes, then fails.
struct Broken(usize);

impl Read for Broken {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.0 == 0 {
            return Err(io::Error::other("the disk went away"));
        }
        let n = buf.len().min(self.0);
        buf[..n].fill(9);
        self.0 -= n;
        Ok(n)
    }
}

#[test]
fn s26_04_a_failed_put_leaves_nothing_behind() {
    let root = Root::new("atomic");
    let store = FsStore::new(&root.0);
    assert!(matches!(
        store.put(&mut Broken(200_000)),
        Err(StoreError::Io(_))
    ));
    assert_eq!(files_under(&root.0), Vec::<PathBuf>::new());
    // A later put of the same prefix still works.
    let mut whole = vec![9_u8; 200_000];
    let hash = store.put(&mut whole.as_slice()).expect("puts");
    whole.clear();
    assert_eq!(read_all(&store, hash).len(), 200_000);
}
