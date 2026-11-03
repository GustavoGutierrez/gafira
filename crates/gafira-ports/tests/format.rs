//! `format` import side (design P4, P8; 02 §4.5; FD-31, ARQ-102).

use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};

use gafira_doc::ContentHash;
use gafira_doc::contract::{CancelToken, Cancelled, NoProgress};
use gafira_doc::msg::Message;
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::common::{Confidence, LegalClass};
use gafira_ports::format::{
    ByteSignature, FormatDescriptor, FormatKind, FormatRegistry, ImportCx, ImportError,
    ImportOptions, Imported, ImportedAsset, Importer,
};
use gafira_ports::registry::RegistryError;
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;

const PNG: &[u8] = &[0x89, b'P', b'N', b'G'];

const fn sig(offset: usize, bytes: &'static [u8]) -> ByteSignature {
    ByteSignature { offset, bytes }
}

fn descriptor(signatures: &'static [ByteSignature]) -> FormatDescriptor {
    FormatDescriptor {
        id: "demo",
        extensions: &["demo"],
        mime: &["image/demo"],
        signatures,
        kind: FormatKind::Raster,
        legal: LegalClass::RoyaltyFree,
    }
}

// FD-31, P8 `ByteSignature::matches`: one row per rule.
#[test]
fn p8_signature_matches_table() {
    let ftyp = sig(4, b"ftyp");
    let cases: Vec<(&str, ByteSignature, &[u8], bool)> = vec![
        ("exact at 0", sig(0, PNG), PNG, true),
        (
            "trailing bytes ignored",
            sig(0, PNG),
            &[0x89, b'P', b'N', b'G', 1, 2],
            true,
        ),
        ("at offset", ftyp, b"\0\0\0\x18ftypmp42", true),
        (
            "last byte differs",
            sig(0, PNG),
            &[0x89, b'P', b'N', b'H'],
            false,
        ),
        (
            "first byte differs",
            sig(0, PNG),
            &[0x88, b'P', b'N', b'G'],
            false,
        ),
        (
            "head one byte short",
            sig(0, PNG),
            &[0x89, b'P', b'N'],
            false,
        ),
        ("head empty", sig(0, PNG), &[], false),
        ("head ends before offset", ftyp, b"\0\0", false),
        ("head ends inside bytes", ftyp, b"\0\0\0\0fty", false),
        ("offset overflows", sig(usize::MAX, PNG), PNG, false),
        (
            "offset + len overflows",
            sig(usize::MAX - 1, PNG),
            PNG,
            false,
        ),
        ("empty bytes, empty head", sig(0, b""), &[], false),
        ("empty bytes, some head", sig(0, b""), PNG, false),
        ("empty bytes, offset past head", sig(99, b""), PNG, false),
    ];
    for (name, signature, head, want) in cases {
        assert_eq!(signature.matches(head), want, "{name}");
    }
}

// FD-31, P8 `sniff_signatures`.
#[test]
fn p8_sniff_signatures_high_on_any_match_else_none() {
    static SIGS: [ByteSignature; 2] = [sig(0, PNG), sig(4, b"ftyp")];
    let d = descriptor(&SIGS);
    assert_eq!(d.sniff_signatures(PNG), Confidence::High, "first signature");
    assert_eq!(
        d.sniff_signatures(b"\0\0\0\0ftyp"),
        Confidence::High,
        "second signature"
    );
    assert_eq!(
        d.sniff_signatures(b"GIF89a"),
        Confidence::None,
        "no signature matches"
    );
    assert_eq!(
        descriptor(&[]).sniff_signatures(PNG),
        Confidence::None,
        "no signatures"
    );
}

// ARQ-102 (ADR 0012, P6): one fixture per variant.
#[test]
fn arq_102_import_error_messages() {
    let hash = ContentHash([0xAB; 32]);
    let own = [
        (ImportError::Unsupported, "error.import.unsupported"),
        (
            ImportError::Corrupt {
                detail: "secret detail".into(),
            },
            "error.import.corrupt",
        ),
        (
            ImportError::Io(std::io::Error::other("secret detail")),
            "error.import.io",
        ),
    ];
    for (error, key) in own {
        let m = error.to_message();
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert_eq!(m.key, MessageKey(key));
        assert!(
            m.args.is_empty(),
            "{key}: library text never becomes an argument"
        );
    }
    // Wrapped errors delegate their message.
    let store = ImportError::Store(StoreError::NotFound { hash }).to_message();
    assert_eq!(store, StoreError::NotFound { hash }.to_message());
    assert_eq!(store.args, vec![("hash", MsgArg::Text("ab".repeat(32)))]);
    let cancelled = ImportError::Cancelled(Cancelled).to_message();
    assert_eq!(cancelled, Cancelled.to_message());
    assert_eq!(cancelled.key, MessageKey("error.job.cancelled"));
}

// ---- doubles ----

#[derive(Clone, Copy)]
enum Act {
    Ok,
    Corrupt,
    Cancel,
}

#[derive(Debug, PartialEq)]
struct Entry {
    id: &'static str,
    pos: u64,
    warnings: usize,
}

type Log = Arc<Mutex<Vec<Entry>>>;

struct Fake {
    d: FormatDescriptor,
    sniff: Confidence,
    act: Act,
    read: usize,
    warn: usize,
    log: Log,
}

impl Importer for Fake {
    fn descriptor(&self) -> &FormatDescriptor {
        &self.d
    }
    fn sniff(&self, _head: &[u8]) -> Confidence {
        self.sniff
    }
    fn import(
        &self,
        src: &mut dyn ReadSeek,
        _opt: &ImportOptions,
        cx: &mut ImportCx<'_>,
    ) -> Result<Imported, ImportError> {
        self.log.lock().unwrap().push(Entry {
            id: self.d.id,
            pos: src.stream_position().unwrap(),
            warnings: cx.warnings.len(),
        });
        let mut buf = vec![0; self.read];
        src.read_exact(&mut buf).unwrap();
        for _ in 0..self.warn {
            cx.warnings.push(gafira_doc::msg!("import.warning"));
        }
        match self.act {
            Act::Ok => Ok(Imported::Asset(ImportedAsset {
                hash: ContentHash([0; 32]),
                mime: self.d.id.into(),
            })),
            Act::Corrupt => Err(ImportError::Corrupt {
                detail: self.d.id.into(),
            }),
            Act::Cancel => Err(ImportError::Cancelled(Cancelled)),
        }
    }
}

struct Spec {
    id: &'static str,
    exts: &'static [&'static str],
    sniff: Confidence,
    act: Act,
    read: usize,
    warn: usize,
}

fn spec(id: &'static str, exts: &'static [&'static str], sniff: Confidence) -> Spec {
    Spec {
        id,
        exts,
        sniff,
        act: Act::Corrupt,
        read: 0,
        warn: 0,
    }
}

impl Spec {
    fn act(mut self, act: Act) -> Self {
        self.act = act;
        self
    }
    fn read(mut self, n: usize) -> Self {
        self.read = n;
        self
    }
    fn warn(mut self, n: usize) -> Self {
        self.warn = n;
        self
    }
}

fn registry(specs: Vec<Spec>) -> (FormatRegistry, Log) {
    let log = Log::default();
    let mut reg = FormatRegistry::new();
    for s in specs {
        let importer = Fake {
            d: FormatDescriptor {
                id: s.id,
                extensions: s.exts,
                mime: &[],
                signatures: &[],
                kind: FormatKind::Raster,
                legal: LegalClass::RoyaltyFree,
            },
            sniff: s.sniff,
            act: s.act,
            read: s.read,
            warn: s.warn,
            log: log.clone(),
        };
        reg.register_importer(Arc::new(importer)).unwrap();
    }
    (reg, log)
}

fn resolved(reg: &FormatRegistry, path: &str) -> Vec<&'static str> {
    reg.resolve_import(Path::new(path), b"head")
        .iter()
        .map(|i| i.descriptor().id)
        .collect()
}

struct NoStore;

impl ContentStore for NoStore {
    fn put(&self, _src: &mut dyn Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }
    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

/// Runs `import` over 16 bytes with one warning already in `cx`.
fn run(
    reg: &FormatRegistry,
    path: &str,
    src: &mut dyn ReadSeek,
) -> (Result<Imported, ImportError>, Vec<Message>) {
    let cancel = CancelToken::new();
    let mut cx = ImportCx {
        store: &NoStore,
        progress: &NoProgress,
        cancel: &cancel,
        warnings: vec![gafira_doc::msg!("import.caller")],
    };
    let r = reg.import(
        Path::new(path),
        b"head",
        src,
        &ImportOptions::default(),
        &mut cx,
    );
    (r, cx.warnings)
}

fn bytes() -> Cursor<Vec<u8>> {
    Cursor::new((0..16).collect())
}

fn mime(r: Result<Imported, ImportError>) -> String {
    match r.unwrap() {
        Imported::Asset(a) => a.mime,
        other => panic!("{other:?}"),
    }
}

fn corrupt(r: Result<Imported, ImportError>) -> String {
    match r.unwrap_err() {
        ImportError::Corrupt { detail } => detail,
        other => panic!("{other:?}"),
    }
}

fn ids(log: &Log) -> Vec<&'static str> {
    log.lock().unwrap().iter().map(|e| e.id).collect()
}

use Confidence::{High, Low, Medium};
const NONE: Confidence = Confidence::None;

// ---- register_importer ----

// ARQ-20: the port is object-safe.
#[test]
fn arq_20_importer_is_object_safe() {
    let (reg, _) = registry(vec![spec("a", &["a"], High)]);
    let first: &dyn Importer = reg.importers().next().unwrap().as_ref();
    assert_eq!(first.descriptor().id, "a");
}

// FD-31, P8 `register_importer`.
#[test]
fn p8_duplicate_importer_id_is_rejected_and_changes_nothing() {
    let (mut reg, log) = registry(vec![spec("a", &["a"], High).act(Act::Ok)]);
    let twin = Fake {
        d: FormatDescriptor {
            id: "a",
            extensions: &["a"],
            mime: &[],
            signatures: &[],
            kind: FormatKind::Vector,
            legal: LegalClass::RoyaltyFree,
        },
        sniff: High,
        act: Act::Corrupt,
        read: 0,
        warn: 0,
        log,
    };
    let err = reg.register_importer(Arc::new(twin)).unwrap_err();
    assert_eq!(err, RegistryError::DuplicateId { id: "a" });
    assert_eq!(reg.importers().count(), 1);
    assert_eq!(
        reg.importers().next().unwrap().descriptor().kind,
        FormatKind::Raster
    );
}

// ---- resolve_import ----

// FD-31 rule 1: the extension is lower-cased ASCII.
#[test]
fn fd_31_extension_is_compared_in_lower_case() {
    let (reg, _) = registry(vec![
        spec("png", &["png"], High),
        spec("other", &["jpg"], Low),
    ]);
    for path in ["a.png", "a.PNG", "a.PnG", "dir.d/a.PNG"] {
        assert_eq!(resolved(&reg, path), ["png"], "{path}");
    }
}

// FD-31 rule 1: the last extension.
#[test]
fn fd_31_last_extension_is_used() {
    let (reg, _) = registry(vec![spec("tar", &["tar"], High), spec("gz", &["gz"], High)]);
    assert_eq!(resolved(&reg, "a.tar.gz"), ["gz"]);
}

// FD-31 rule 1: no extension matches nothing, so the fallback of rule 3 runs.
#[test]
fn fd_31_missing_extension_matches_no_importer() {
    let (reg, _) = registry(vec![
        spec("png", &["png"], High),
        spec("jpg", &["jpg"], Medium),
    ]);
    for path in ["README", ".png", "dir.png/README", "a."] {
        assert_eq!(resolved(&reg, path), ["png", "jpg"], "{path}");
    }
}

// FD-31 rule 1: a non-UTF-8 extension matches no importer.
#[cfg(unix)]
#[test]
fn fd_31_non_utf8_extension_matches_no_importer() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let (reg, _) = registry(vec![
        spec("png", &["png"], Medium),
        spec("jpg", &["jpg"], High),
    ]);
    let path = Path::new(OsStr::from_bytes(b"a.\xff\xfe"));
    let got: Vec<_> = reg
        .resolve_import(path, b"head")
        .iter()
        .map(|i| i.descriptor().id)
        .collect();
    assert_eq!(got, ["jpg", "png"]);
}

// FD-31 rule 2: a `None` sniff is dropped, with or without the fallback.
#[test]
fn fd_31_none_sniff_is_dropped() {
    let (reg, _) = registry(vec![spec("a", &["png"], NONE), spec("b", &["png"], Low)]);
    assert_eq!(resolved(&reg, "x.png"), ["b"]);

    let (reg, _) = registry(vec![
        spec("a", &["png"], NONE),
        spec("b", &["jpg"], Low),
        spec("c", &["gif"], NONE),
        spec("d", &["gif"], High),
    ]);
    assert_eq!(
        resolved(&reg, "x.xyz"),
        ["d", "b"],
        "fallback drops None too"
    );
}

// FD-31 rule 2 and 3: nobody recognises the head, so nobody is tried.
#[test]
fn fd_31_all_none_gives_an_empty_list() {
    let (reg, _) = registry(vec![spec("a", &["png"], NONE), spec("b", &["jpg"], NONE)]);
    assert!(resolved(&reg, "x.png").is_empty());
    assert!(resolved(&reg, "x.xyz").is_empty());
}

// FD-31 rule 3 (S3): a PNG named `.jpg` still opens.
#[test]
fn fd_31_fallback_tries_every_importer_when_the_extension_has_none() {
    let (reg, _) = registry(vec![
        spec("png", &["png"], High),
        spec("jpg", &["jpg"], NONE),
    ]);
    assert_eq!(resolved(&reg, "x.jpg"), ["png"]);
}

// FD-31 rule 2 before 3: extension matches win over a better sniff elsewhere.
#[test]
fn fd_31_extension_filter_takes_priority_over_confidence() {
    let (reg, _) = registry(vec![spec("a", &["png"], Low), spec("b", &["jpg"], High)]);
    assert_eq!(resolved(&reg, "x.png"), ["a"]);
}

// FD-31 rule 4: highest confidence first.
#[test]
fn fd_31_sorted_by_confidence_descending() {
    let (reg, _) = registry(vec![
        spec("low", &["x"], Low),
        spec("high", &["x"], High),
        spec("mid", &["x"], Medium),
    ]);
    assert_eq!(resolved(&reg, "a.x"), ["high", "mid", "low"]);
    assert_eq!(
        resolved(&reg, "a.y"),
        ["high", "mid", "low"],
        "fallback too"
    );
}

// FD-31 rule 4: ties keep registration order.
#[test]
fn fd_31_tie_keeps_registration_order() {
    let (reg, _) = registry(vec![
        spec("c", &["x"], High),
        spec("a", &["x"], High),
        spec("b", &["x"], High),
    ]);
    assert_eq!(resolved(&reg, "f.x"), ["c", "a", "b"]);
}

// FD-31 rule 4 on a list long enough that an unstable sort reorders ties.
#[test]
fn fd_31_tie_order_holds_on_a_long_list() {
    const IDS: [&str; 64] = [
        "i00", "i01", "i02", "i03", "i04", "i05", "i06", "i07", "i08", "i09", "i10", "i11", "i12",
        "i13", "i14", "i15", "i16", "i17", "i18", "i19", "i20", "i21", "i22", "i23", "i24", "i25",
        "i26", "i27", "i28", "i29", "i30", "i31", "i32", "i33", "i34", "i35", "i36", "i37", "i38",
        "i39", "i40", "i41", "i42", "i43", "i44", "i45", "i46", "i47", "i48", "i49", "i50", "i51",
        "i52", "i53", "i54", "i55", "i56", "i57", "i58", "i59", "i60", "i61", "i62", "i63",
    ];
    let level = |i: usize| [Low, High, Medium][(i * 7 + i / 5) % 3];
    let specs = IDS
        .iter()
        .enumerate()
        .map(|(i, id)| spec(id, &["x"], level(i)))
        .collect();
    let (reg, _) = registry(specs);
    let mut want: Vec<usize> = (0..IDS.len()).collect();
    want.sort_by(|&a, &b| level(b).cmp(&level(a)).then(a.cmp(&b)));
    let want: Vec<_> = want.into_iter().map(|i| IDS[i]).collect();
    assert_eq!(resolved(&reg, "f.x"), want);
}

// ---- import ----

// FD-31 import rule 1.
#[test]
fn fd_31_import_without_candidates_is_unsupported() {
    let (reg, log) = registry(vec![]);
    let (r, _) = run(&reg, "a.png", &mut bytes());
    assert!(matches!(r, Err(ImportError::Unsupported)), "{r:?}");

    let (reg, log2) = registry(vec![spec("a", &["png"], NONE)]);
    let (r, _) = run(&reg, "a.png", &mut bytes());
    assert!(matches!(r, Err(ImportError::Unsupported)), "{r:?}");
    assert!(ids(&log).is_empty() && ids(&log2).is_empty());
}

// FD-31 import rule 3: the first Ok wins and nothing else runs.
#[test]
fn fd_31_first_ok_is_the_result() {
    let (reg, log) = registry(vec![
        spec("a", &["x"], Low).act(Act::Ok),
        spec("b", &["x"], High).act(Act::Ok),
    ]);
    let (r, _) = run(&reg, "f.x", &mut bytes());
    assert_eq!(mime(r), "b");
    assert_eq!(ids(&log), ["b"]);
}

// FD-31 import rule 5: another error moves to the next importer.
#[test]
fn fd_31_failure_moves_to_the_next_importer() {
    let (reg, log) = registry(vec![
        spec("a", &["x"], High),
        spec("b", &["x"], Medium).act(Act::Ok),
        spec("c", &["x"], Low).act(Act::Ok),
    ]);
    let (r, _) = run(&reg, "f.x", &mut bytes());
    assert_eq!(mime(r), "b");
    assert_eq!(ids(&log), ["a", "b"]);
}

// FD-31 import rule 2: every try starts at byte 0.
#[test]
fn fd_31_each_try_reads_from_byte_zero() {
    let (reg, log) = registry(vec![
        spec("a", &["x"], High).read(5),
        spec("b", &["x"], Medium).read(9),
        spec("c", &["x"], Low).read(1).act(Act::Ok),
    ]);
    let mut src = bytes();
    src.seek(SeekFrom::Start(3)).unwrap();
    let (r, _) = run(&reg, "f.x", &mut src);
    assert_eq!(mime(r), "c");
    let pos: Vec<u64> = log.lock().unwrap().iter().map(|e| e.pos).collect();
    assert_eq!(pos, [0, 0, 0]);
}

// FD-31 import rule 2: a failed seek is an `Io` error and nothing runs.
struct SeekFails {
    inner: Cursor<Vec<u8>>,
    seeks_ok: usize,
}

impl Read for SeekFails {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for SeekFails {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        // Only rewinds count: the double asks for its position with `Current`.
        if pos == SeekFrom::Start(0) {
            if self.seeks_ok == 0 {
                return Err(io::Error::other("seek refused"));
            }
            self.seeks_ok -= 1;
        }
        self.inner.seek(pos)
    }
}

#[test]
fn fd_31_failed_seek_is_an_io_error() {
    for seeks_ok in [0, 1] {
        let (reg, log) = registry(vec![
            spec("a", &["x"], High),
            spec("b", &["x"], Medium).act(Act::Ok),
        ]);
        let mut src = SeekFails {
            inner: bytes(),
            seeks_ok,
        };
        let (r, _) = run(&reg, "f.x", &mut src);
        assert!(matches!(r, Err(ImportError::Io(_))), "{seeks_ok}: {r:?}");
        assert_eq!(
            ids(&log).len(),
            seeks_ok,
            "no importer runs after the failed seek"
        );
    }
}

// FD-31 import rule 4: `Cancelled` stops everything.
#[test]
fn fd_31_cancelled_stops_the_tries() {
    let (reg, log) = registry(vec![
        spec("a", &["x"], High).act(Act::Cancel),
        spec("b", &["x"], Medium).act(Act::Ok),
    ]);
    let (r, _) = run(&reg, "f.x", &mut bytes());
    assert!(matches!(r, Err(ImportError::Cancelled(_))), "{r:?}");
    assert_eq!(ids(&log), ["a"]);

    // After an earlier failure it is `Cancelled` that comes back, not that failure.
    let (reg, log) = registry(vec![
        spec("a", &["x"], High),
        spec("b", &["x"], Medium).act(Act::Cancel),
        spec("c", &["x"], Low).act(Act::Ok),
    ]);
    let (r, _) = run(&reg, "f.x", &mut bytes());
    assert!(matches!(r, Err(ImportError::Cancelled(_))), "{r:?}");
    assert_eq!(ids(&log), ["a", "b"]);
}

// FD-31 import rule 5: all fail, the first error comes back.
#[test]
fn fd_31_when_all_fail_the_first_error_is_returned() {
    let (reg, log) = registry(vec![
        spec("a", &["x"], High),
        spec("b", &["x"], Medium),
        spec("c", &["x"], Low),
    ]);
    let (r, _) = run(&reg, "f.x", &mut bytes());
    assert_eq!(corrupt(r), "a");
    assert_eq!(ids(&log), ["a", "b", "c"]);
}

// FD-31 import rule 2: the warnings of a failed try are discarded, the
// caller's are kept.
#[test]
fn fd_31_warnings_of_a_failed_try_are_discarded() {
    let (reg, log) = registry(vec![
        spec("a", &["x"], High).warn(2),
        spec("b", &["x"], Medium).warn(1),
        spec("c", &["x"], Low).warn(1).act(Act::Ok),
    ]);
    let (r, warnings) = run(&reg, "f.x", &mut bytes());
    assert_eq!(mime(r), "c");
    let seen: Vec<usize> = log.lock().unwrap().iter().map(|e| e.warnings).collect();
    assert_eq!(
        seen,
        [1, 1, 1],
        "each try starts with the caller's warning only"
    );
    assert_eq!(warnings.len(), 2, "the caller's plus the successful try's");
    assert_eq!(warnings[0].key.0, "import.caller");
}

// ADR 0012 rule 4 via ADR 0014: `?` turns a `Cancelled` into the variant.
#[test]
fn adr_0012_import_error_from_cancelled() {
    let e: ImportError = Cancelled.into();
    assert!(matches!(e, ImportError::Cancelled(_)), "{e:?}");
}

// 02 §4.5 rule 5: a failed import leaves `cx.warnings` as it was at the call,
// on every `Err` return.
#[test]
fn fd_31_warnings_are_restored_when_every_importer_fails() {
    let (reg, _) = registry(vec![
        spec("a", &["x"], High).warn(2),
        spec("b", &["x"], Medium).warn(1),
    ]);
    let (r, warnings) = run(&reg, "f.x", &mut bytes());
    assert_eq!(corrupt(r), "a");
    assert_eq!(warnings.len(), 1, "only the caller's warning is left");
    assert_eq!(warnings[0].key.0, "import.caller");
}

#[test]
fn fd_31_warnings_are_restored_when_an_importer_cancels() {
    let (reg, _) = registry(vec![
        spec("a", &["x"], High).warn(2),
        spec("b", &["x"], Medium).warn(1).act(Act::Cancel),
    ]);
    let (r, warnings) = run(&reg, "f.x", &mut bytes());
    assert!(matches!(r, Err(ImportError::Cancelled(_))), "{r:?}");
    assert_eq!(warnings.len(), 1, "only the caller's warning is left");
    assert_eq!(warnings[0].key.0, "import.caller");
}

#[test]
fn fd_31_warnings_are_restored_when_the_rewind_fails_after_a_try() {
    let (reg, _) = registry(vec![
        spec("a", &["x"], High).warn(2),
        spec("b", &["x"], Medium).act(Act::Ok),
    ]);
    let mut src = SeekFails {
        inner: bytes(),
        seeks_ok: 1,
    };
    let (r, warnings) = run(&reg, "f.x", &mut src);
    assert!(matches!(r, Err(ImportError::Io(_))), "{r:?}");
    assert_eq!(warnings.len(), 1, "only the caller's warning is left");
}
