//! Spec 033: the thumbnail of the first page in `thumbs/` (02 §7.1, D-68)
//! and reading it without opening the project (03 §7.2).

use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};
use std::sync::Mutex;

use gafira_doc::{AssetRef, ContentHash, Document, PageId, ProjectId};
use gafira_format::{GafRepository, Limits, ThumbnailImage, read_gaf, read_thumbnail, write_gaf};
use gafira_ports::repository::{ProjectRepository, RepositoryError, THUMBNAIL_SIDE, Thumbnail};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;
use gafira_raster::{ImageBuf, PixelBuf};
use serde_json::{Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Contents kept in memory.
#[derive(Default)]
struct MemoryStore(Mutex<BTreeMap<ContentHash, Vec<u8>>>);

impl ContentStore for MemoryStore {
    fn put(&self, src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        let mut bytes = Vec::new();
        src.read_to_end(&mut bytes).map_err(StoreError::Io)?;
        let hash = ContentHash(*blake3::hash(&bytes).as_bytes());
        self.0.lock().expect("a lock").insert(hash, bytes);
        Ok(hash)
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        let bytes = self.0.lock().expect("a lock").get(&hash).cloned();
        let bytes = bytes.ok_or(StoreError::NotFound { hash })?;
        Ok(Box::new(Cursor::new(bytes)))
    }

    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        Ok(self.0.lock().expect("a lock").contains_key(&hash))
    }
}

/// The bytes of the embedded `clip.mp4` of the golden.
const CLIP: &[u8] = b"not really an mp4";

fn store() -> MemoryStore {
    let store = MemoryStore::default();
    store.put(&mut &CLIP[..]).expect("puts");
    store
}

/// The timeline golden: one page (id 1), a sequence and the clip.
fn document() -> Document {
    let hex = AssetRef::new(ContentHash(*blake3::hash(CLIP).as_bytes())).hex();
    let text = include_str!("../../gafira-doc/tests/golden/timeline.json").replace(
        "abababababababababababababababababababababababababababababababab",
        &hex,
    );
    Document::from_json(text.as_bytes())
        .expect("the golden loads")
        .with_project_id(ProjectId::from_u128(0xfeed))
}

/// The straight pixels of [`thumbnail`]: four colours, one half covered.
const PIXELS: [u8; 16] = [
    255, 0, 0, 255, //
    0, 255, 0, 255, //
    0, 0, 255, 255, //
    199, 100, 0, 128,
];

/// A 2 × 2 preview render (premultiplied sRGB bytes) of page 1.
fn thumbnail() -> Thumbnail {
    let premultiplied = vec![
        255, 0, 0, 255, //
        0, 255, 0, 255, //
        0, 0, 255, 255, //
        100, 50, 0, 128,
    ];
    Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(2, 2, PixelBuf::Rgba8(premultiplied)).expect("an image"),
    }
}

fn gaf(thumbnail: Option<&Thumbnail>) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    write_gaf(&mut bytes, &document(), &[], thumbnail, &store()).expect("writes");
    bytes.into_inner()
}

fn thumb(bytes: Vec<u8>) -> Result<Option<ThumbnailImage>, RepositoryError> {
    read_thumbnail(Cursor::new(bytes), Limits::default())
}

fn unpacked(bytes: &[u8]) -> Vec<(String, CompressionMethod, Vec<u8>)> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    (0..zip.len())
        .map(|index| {
            let mut file = zip.by_index(index).expect("an entry");
            let mut data = Vec::new();
            file.read_to_end(&mut data).expect("bytes");
            (file.name().to_owned(), file.compression(), data)
        })
        .collect()
}

fn packed(parts: &[(String, CompressionMethod, Vec<u8>)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, method, data) in parts {
        zip.start_file(
            name.as_str(),
            SimpleFileOptions::default().compression_method(*method),
        )
        .expect("an entry");
        zip.write_all(data).expect("bytes");
    }
    zip.finish().expect("a zip").into_inner()
}

/// The file with a thumbnail, each entry passed through `edit`; `None`
/// drops the entry.
fn edited(edit: impl Fn(String, Vec<u8>) -> Option<(String, Vec<u8>)>) -> Vec<u8> {
    let parts: Vec<_> = unpacked(&gaf(Some(&thumbnail())))
        .into_iter()
        .filter_map(|(name, method, data)| {
            edit(name, data).map(|(name, data)| (name, method, data))
        })
        .collect();
    packed(&parts)
}

/// A lossless WebP of `width` × `height` opaque white pixels.
fn webp(width: u32, height: u32) -> Vec<u8> {
    let pixels = vec![255; (width * height * 4) as usize];
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(&pixels, width, height, image_webp::ColorType::Rgba8)
        .expect("encodes");
    out
}

fn corrupt<T: std::fmt::Debug>(result: Result<T, RepositoryError>) -> bool {
    matches!(result, Err(RepositoryError::Corrupt { .. }))
}

// S33-02: the thumbnail is stored as a lossless WebP under the page's id
// and comes back with the same straight pixels.
#[test]
fn s33_02_the_thumbnail_round_trips_as_thumbs_page_id_webp() {
    let bytes = gaf(Some(&thumbnail()));
    let parts = unpacked(&bytes);
    let (_, method, data) = parts
        .iter()
        .find(|(name, _, _)| name == "thumbs/1.webp")
        .expect("thumbs/1.webp");
    assert_eq!(*method, CompressionMethod::Stored, "WebP is compressed");
    assert_eq!(&data[..4], b"RIFF");
    assert_eq!(&data[8..12], b"WEBP");
    let read = thumb(bytes.clone()).expect("reads").expect("a thumbnail");
    assert_eq!((read.width, read.height), (2, 2));
    assert_eq!(read.rgba, PIXELS);
    // The project still opens as before.
    let loaded = read_gaf(Cursor::new(bytes.clone()), Limits::default(), &store()).expect("opens");
    assert_eq!(loaded.document, document());
    assert_eq!(gaf(Some(&thumbnail())), bytes, "deterministic");
}

// S33-02: without a thumbnail the file is the one of spec 019.
#[test]
fn s33_02_without_a_thumbnail_there_is_no_thumbs_entry() {
    let bytes = gaf(None);
    assert!(
        unpacked(&bytes)
            .iter()
            .all(|(name, _, _)| !name.starts_with("thumbs/"))
    );
    assert_eq!(thumb(bytes).expect("reads"), None);
}

// S33-02: a thumbnail the format cannot store is left out; the save goes on.
#[test]
fn s33_02_an_unwritable_thumbnail_is_left_out() {
    let mask = Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(1, 1, PixelBuf::Alpha8(vec![255])).expect("a mask"),
    };
    let wide = Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(THUMBNAIL_SIDE + 1, 1, PixelBuf::Rgba8(vec![255; 513 * 4]))
            .expect("an image"),
    };
    let empty = Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(0, 0, PixelBuf::Rgba8(Vec::new())).expect("an empty image"),
    };
    for bad in [mask, wide, empty] {
        assert_eq!(gaf(Some(&bad)), gaf(None), "{:?}", bad.image.width());
    }
    let largest = Thumbnail {
        page: PageId::new(1),
        image: ImageBuf::new(THUMBNAIL_SIDE, 1, PixelBuf::Rgba8(vec![255; 512 * 4]))
            .expect("an image"),
    };
    let read = thumb(gaf(Some(&largest))).expect("reads").expect("kept");
    assert_eq!((read.width, read.height), (THUMBNAIL_SIDE, 1));
}

// S33-03: opening a file treats `thumbs/` as hostile: the name must be
// `<number>.webp`.
#[test]
fn s33_03_a_misnamed_thumbs_entry_is_corrupt() {
    for bad in [
        "thumbs/one.webp",
        "thumbs/1.png",
        "thumbs/1",
        "thumbs/a/1.webp",
    ] {
        let bytes = edited(|name, data| {
            let name = if name == "thumbs/1.webp" {
                bad.to_owned()
            } else {
                name
            };
            Some((name, data))
        });
        assert!(
            corrupt(read_gaf(Cursor::new(bytes), Limits::default(), &store())),
            "{bad}"
        );
    }
}

// S33-03: and it must stay under the cap.
#[test]
fn s33_03_an_oversized_thumbs_entry_is_corrupt() {
    let bytes = gaf(Some(&thumbnail()));
    let limits = Limits {
        max_thumb_bytes: 16,
        ..Limits::default()
    };
    assert!(corrupt(read_gaf(
        Cursor::new(bytes.clone()),
        limits,
        &store()
    )));
    assert!(corrupt(read_thumbnail(Cursor::new(bytes.clone()), limits)));
    // Exactly at the cap is fine.
    let size = unpacked(&bytes)
        .into_iter()
        .find(|(name, _, _)| name == "thumbs/1.webp")
        .map(|(_, _, data)| data.len() as u64)
        .expect("thumbs/1.webp");
    let exact = Limits {
        max_thumb_bytes: size,
        ..Limits::default()
    };
    assert!(read_gaf(Cursor::new(bytes.clone()), exact, &store()).is_ok());
    assert!(
        read_thumbnail(Cursor::new(bytes), exact)
            .expect("reads")
            .is_some()
    );
}

// S33-04: the thumbnail reads with only the mimetype, the manifest and its
// own entry: a broken document does not stop it.
#[test]
fn s33_04_the_thumbnail_reads_without_the_document() {
    let bytes = edited(|name, data| match name.as_str() {
        "document.json" => Some((name, b"not json".to_vec())),
        "pages/1.json" => None,
        _ => Some((name, data)),
    });
    assert!(corrupt(read_gaf(
        Cursor::new(bytes.clone()),
        Limits::default(),
        &store()
    )));
    let read = thumb(bytes).expect("reads").expect("a thumbnail");
    assert_eq!(read.rgba, PIXELS);
}

// S33-04: the first page the manifest lists names the thumbnail; with no
// page, or no entry, there is none.
#[test]
fn s33_04_no_page_or_no_entry_is_no_thumbnail() {
    let no_pages = edited(|name, data| {
        if name == "manifest.json" {
            let mut manifest: Value = serde_json::from_slice(&data).expect("json");
            manifest["pages"] = json!([]);
            Some((name, serde_json::to_vec(&manifest).expect("json")))
        } else {
            Some((name, data))
        }
    });
    assert_eq!(thumb(no_pages).expect("reads"), None);
    let other_page = edited(|name, data| {
        let name = if name == "thumbs/1.webp" {
            "thumbs/2.webp".to_owned()
        } else {
            name
        };
        Some((name, data))
    });
    assert_eq!(thumb(other_page).expect("reads"), None);
}

// S33-04: the cheap read is still a hostile read.
#[test]
fn s33_04_a_hostile_thumbnail_is_corrupt() {
    let with = |bytes: Vec<u8>| {
        edited(move |name, data| {
            if name == "thumbs/1.webp" {
                Some((name, bytes.clone()))
            } else {
                Some((name, data))
            }
        })
    };
    assert!(
        corrupt(thumb(with(b"RIFF....WEBPnope".to_vec()))),
        "not a WebP"
    );
    assert!(
        corrupt(thumb(with(webp(THUMBNAIL_SIDE + 1, 1)))),
        "too wide"
    );
    assert!(
        corrupt(thumb(with(webp(1, THUMBNAIL_SIDE + 1)))),
        "too tall"
    );
    assert_eq!(
        thumb(with(webp(THUMBNAIL_SIDE, 3)))
            .expect("reads")
            .map(|t| t.width),
        Some(THUMBNAIL_SIDE)
    );
    let foreign = edited(|name, data| {
        if name == "manifest.json" {
            let mut manifest: Value = serde_json::from_slice(&data).expect("json");
            manifest["format"] = json!("something-else");
            Some((name, serde_json::to_vec(&manifest).expect("json")))
        } else {
            Some((name, data))
        }
    });
    assert!(corrupt(thumb(foreign)), "not a gafira-project");
    let no_mimetype = edited(|name, data| (name != "mimetype").then_some((name, data)));
    assert!(corrupt(thumb(no_mimetype)), "no mimetype first");
    assert!(corrupt(thumb(b"not a zip".to_vec())), "not a zip");
}

// S33-01, S33-04: the repository writes the thumbnail into the file on
// disk, and it reads back from that file alone.
#[test]
fn s33_04_the_repository_saves_a_thumbnail_a_file_reader_finds() {
    let dir = std::env::temp_dir().join(format!("gafira-thumbs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("poster.gaf");
    let repository = GafRepository::new(Limits::default(), std::sync::Arc::new(store()));
    repository
        .save_with_thumbnail(&path, &document(), &[], Some(&thumbnail()))
        .expect("saves");
    let file = io::BufReader::new(std::fs::File::open(&path).expect("opens"));
    let read = read_thumbnail(file, Limits::default())
        .expect("reads")
        .expect("a thumbnail");
    assert_eq!(read.rgba, PIXELS);
    repository.save(&path, &document(), &[]).expect("saves");
    let file = io::BufReader::new(std::fs::File::open(&path).expect("opens"));
    assert_eq!(
        read_thumbnail(file, Limits::default()).expect("reads"),
        None
    );
    std::fs::remove_dir_all(dir).expect("clean up");
}
