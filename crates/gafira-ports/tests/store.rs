//! `ContentStore` and `StoreError` (design P4, P6).

use std::io::{Cursor, Read};
use std::sync::Mutex;

use gafira_doc::ContentHash;
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;

/// A store double that keeps one blob.
struct OneBlob(Mutex<Option<(ContentHash, Vec<u8>)>>);

impl ContentStore for OneBlob {
    fn put(&self, src: &mut dyn Read) -> Result<ContentHash, StoreError> {
        let mut bytes = Vec::new();
        src.read_to_end(&mut bytes).map_err(StoreError::Io)?;
        let hash = ContentHash([bytes.len() as u8; 32]);
        *self.0.lock().unwrap() = Some((hash, bytes));
        Ok(hash)
    }
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        match &*self.0.lock().unwrap() {
            Some((h, bytes)) if *h == hash => Ok(Box::new(Cursor::new(bytes.clone()))),
            _ => Err(StoreError::NotFound { hash }),
        }
    }
    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        Ok(matches!(&*self.0.lock().unwrap(), Some((h, _)) if *h == hash))
    }
}

// ARQ-20: the port is object-safe and a double runs through `dyn`.
#[test]
fn arq_20_content_store_is_object_safe() {
    let store: Box<dyn ContentStore> = Box::new(OneBlob(Mutex::new(None)));
    let hash = store.put(&mut Cursor::new(b"abc".to_vec())).unwrap();
    assert!(store.contains(hash).unwrap());
    let mut back = String::new();
    store.open(hash).unwrap().read_to_string(&mut back).unwrap();
    assert_eq!(back, "abc");
    let missing = ContentHash([9; 32]);
    assert!(!store.contains(missing).unwrap());
    assert!(matches!(
        store.open(missing),
        Err(StoreError::NotFound { hash }) if hash == missing
    ));
}

// ARQ-102 (ADR 0012): one fixture per variant.
#[test]
fn arq_102_store_error_messages() {
    let not_found = StoreError::NotFound {
        hash: ContentHash([0xAB; 32]),
    }
    .to_message();
    let io = StoreError::Io(std::io::Error::other("secret detail")).to_message();
    for m in [&not_found, &io] {
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert!(m.key.0.starts_with("error.store."));
    }
    assert_eq!(not_found.key, MessageKey("error.store.not_found"));
    assert_eq!(
        not_found.args,
        vec![("hash", MsgArg::Text("ab".repeat(32)))]
    );
    assert_eq!(io.key, MessageKey("error.store.io"));
    assert!(io.args.is_empty(), "library text never becomes an argument");
}
