//! `FileSystem`, `Clock`, `Notifier` and the seek traits (design P4).

use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gafira_ports::common::UnixTime;
use gafira_ports::system::{Clock, FileSystem, FileWrite, Notifier, ReadSeek, WriteSeek};

type Trace = Arc<Mutex<Vec<String>>>;

/// A file system double that records every call it gets.
struct Recording {
    trace: Trace,
}

struct RecordedFile {
    trace: Trace,
    buf: Cursor<Vec<u8>>,
}

impl Write for RecordedFile {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.trace
            .lock()
            .unwrap()
            .push(format!("write {}", data.len()));
        self.buf.write(data)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.buf.flush()
    }
}

impl Seek for RecordedFile {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.buf.seek(to)
    }
}

impl FileWrite for RecordedFile {
    fn sync_all(&mut self) -> std::io::Result<()> {
        self.trace.lock().unwrap().push("sync".into());
        Ok(())
    }
}

impl Recording {
    fn log(&self, line: String) {
        self.trace.lock().unwrap().push(line);
    }
}

impl FileSystem for Recording {
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn ReadSeek + Send>> {
        self.log(format!("open {}", path.display()));
        Ok(Box::new(Cursor::new(Vec::new())))
    }
    fn create(&self, path: &Path) -> std::io::Result<Box<dyn FileWrite>> {
        self.log(format!("create {}", path.display()));
        Ok(Box::new(RecordedFile {
            trace: Arc::clone(&self.trace),
            buf: Cursor::new(Vec::new()),
        }))
    }
    fn replace(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.log(format!("replace {} {}", from.display(), to.display()));
        Ok(())
    }
    fn remove(&self, path: &Path) -> std::io::Result<()> {
        self.log(format!("remove {}", path.display()));
        Ok(())
    }
    fn exists(&self, path: &Path) -> std::io::Result<bool> {
        self.log(format!("exists {}", path.display()));
        Ok(false)
    }
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.log(format!("mkdir {}", path.display()));
        Ok(())
    }
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        self.log(format!("read_dir {}", path.display()));
        Ok(vec![path.join("a")])
    }
}

/// The save of 02 §7.2: temp file, write, sync, atomic replace.
fn save(fs: &dyn FileSystem, target: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = target.with_extension("tmp");
    let mut file = fs.create(&tmp)?;
    file.write_all(data)?;
    file.sync_all()?;
    drop(file);
    fs.replace(&tmp, target)
}

// ARQ-50: the port can express the write, sync and replace trace, in order.
#[test]
fn arq_50_file_system_expresses_the_atomic_save_trace() {
    let trace: Trace = Arc::default();
    let fs: Box<dyn FileSystem> = Box::new(Recording {
        trace: Arc::clone(&trace),
    });
    save(&*fs, Path::new("p/doc.gafira"), b"hello").unwrap();
    assert_eq!(
        *trace.lock().unwrap(),
        [
            "create p/doc.tmp",
            "write 5",
            "sync",
            "replace p/doc.tmp p/doc.gafira"
        ]
    );
}

// ARQ-50: the other operations are reachable through `dyn FileSystem`.
#[test]
fn arq_50_file_system_other_operations_run_through_dyn() {
    let trace: Trace = Arc::default();
    let fs: &dyn FileSystem = &Recording {
        trace: Arc::clone(&trace),
    };
    fs.create_dir_all(Path::new("d")).unwrap();
    assert!(!fs.exists(Path::new("d")).unwrap());
    assert_eq!(fs.read_dir(Path::new("d")).unwrap(), [PathBuf::from("d/a")]);
    fs.remove(Path::new("d/a")).unwrap();
    let mut src = fs.open(Path::new("d/a")).unwrap();
    assert_eq!(src.read(&mut [0; 4]).unwrap(), 0);
    assert_eq!(trace.lock().unwrap().len(), 5);
}

// P4: any `Read + Seek` or `Write + Seek` type is a port stream, sized or not.
#[test]
fn p4_blanket_impls_cover_std_streams() {
    let mut read: Box<dyn ReadSeek + Send> = Box::new(Cursor::new(b"abcdef".to_vec()));
    read.seek(SeekFrom::Start(2)).unwrap();
    let mut rest = String::new();
    read.read_to_string(&mut rest).unwrap();
    assert_eq!(rest, "cdef");

    let mut write: Box<dyn WriteSeek + Send> = Box::new(Cursor::new(Vec::new()));
    write.write_all(b"xyz").unwrap();
    write.seek(SeekFrom::Start(1)).unwrap();
    write.write_all(b"!").unwrap();
    write.seek(SeekFrom::Start(0)).unwrap();

    // An unsized stream type is a port stream too (the `?Sized` bound).
    trait Wider: Read + Seek + Write {}
    impl<T: Read + Seek + Write> Wider for T {}
    fn end_of<S: ReadSeek + ?Sized>(src: &mut S) -> u64 {
        src.seek(SeekFrom::End(0)).unwrap()
    }
    fn end_of_out<S: WriteSeek + ?Sized>(dst: &mut S) -> u64 {
        dst.seek(SeekFrom::End(0)).unwrap()
    }
    let mut wide: Box<dyn Wider> = Box::new(Cursor::new(vec![0u8; 7]));
    assert_eq!(end_of(&mut *wide), 7);
    assert_eq!(end_of_out(&mut *wide), 7);
}

/// A clock double a test moves by hand.
struct Manual {
    secs: AtomicI64,
}

impl Clock for Manual {
    fn now(&self) -> UnixTime {
        UnixTime(self.secs.load(Ordering::SeqCst))
    }
    fn monotonic(&self) -> Duration {
        Duration::from_secs(u64::try_from(self.secs.load(Ordering::SeqCst)).unwrap())
    }
}

// ARQ-93: a component takes `&dyn Clock`, so a test decides what time it is.
#[test]
fn arq_93_clock_double_is_injectable() {
    fn grace_left(clock: &dyn Clock, until: UnixTime) -> i64 {
        until.0 - clock.now().0
    }
    let clock = Arc::new(Manual {
        secs: AtomicI64::new(1_000),
    });
    let shared: Arc<dyn Clock> = clock.clone();
    assert_eq!(grace_left(&*shared, UnixTime(1_600)), 600);
    clock.secs.store(1_700, Ordering::SeqCst);
    assert_eq!(grace_left(&*shared, UnixTime(1_600)), -100);
    assert_eq!(shared.monotonic(), Duration::from_secs(1_700));
}

// ARQ-20: the notifier is object-safe.
#[test]
fn arq_20_notifier_is_object_safe() {
    struct Sink(Mutex<Vec<(String, String)>>);
    impl Notifier for Sink {
        fn notify(&self, title: &str, body: &str) {
            self.0.lock().unwrap().push((title.into(), body.into()));
        }
    }
    let sink = Arc::new(Sink(Mutex::default()));
    let dynamic: Arc<dyn Notifier> = sink.clone();
    dynamic.notify("done", "export finished");
    assert_eq!(
        *sink.0.lock().unwrap(),
        [("done".to_string(), "export finished".to_string())]
    );
}
