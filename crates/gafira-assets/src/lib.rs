//! Layer-4 adapter `gafira-assets` (02 §2.1, §5.3; spec 026): the
//! content-addressed store on disk that implements `ports::ContentStore`.

#![forbid(unsafe_code)]

mod store;

pub use store::FsStore;
