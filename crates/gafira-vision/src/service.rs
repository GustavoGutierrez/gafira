//! The models of this machine (S84-04, S84-05): the one chosen for each task
//! is loaded once, on the runtime it is fastest on.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::vision::{
    BackgroundRemover, Machine, ModelInfo, Upscaler, VisionError, VisionModels, VisionTask,
};

use rayon::prelude::*;

use crate::catalog::{
    MODELS, Model, ONNX_RUNTIME, Shape, default_of, library_is_for, model, right_for,
};
use crate::cutout::Cutter;
use crate::fetch::Fetch;
use crate::runtime::tract::Tract;
use crate::runtime::{Runtime, Session};
use crate::store::{Settings, Store};
use crate::upscale::Enlarger;

/// A model in memory.
struct Loaded {
    id: &'static str,
    session: Arc<dyn Session>,
}

#[derive(Default)]
struct State {
    settings: Settings,
    /// One for each task, the last one used.
    cutout: Option<Loaded>,
    upscale: Option<Loaded>,
}

/// The adapter of [`VisionModels`].
pub struct Vision {
    machine: Machine,
    store: Store,
    fetch: Box<dyn Fetch>,
    state: Mutex<State>,
    /// Loading takes a while: another that asks waits for it, and the state
    /// can be read meanwhile.
    loading: Mutex<()>,
}

fn name(task: VisionTask) -> &'static str {
    match task {
        VisionTask::Cutout => "cutout",
        VisionTask::Upscale => "upscale",
    }
}

impl Vision {
    /// The models kept in `folder`, downloaded with `fetch`, on `machine`.
    #[must_use]
    pub fn new(folder: PathBuf, fetch: Box<dyn Fetch>, machine: Machine) -> Self {
        let store = Store::at(folder);
        let state = State {
            settings: store.settings(),
            ..State::default()
        };
        Self {
            machine,
            store,
            fetch,
            state: Mutex::new(state),
            loading: Mutex::new(()),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The ways to run a model here, the surest last.
    fn runtimes_here(&self) -> Vec<Box<dyn Runtime>> {
        let mut out: Vec<Box<dyn Runtime>> = Vec::new();
        #[cfg(feature = "onnx")]
        if let Some(file) = ONNX_RUNTIME
            .as_ref()
            .and_then(|library| self.store.library_file(library))
        {
            use crate::runtime::onnx::{Device, Onnx};
            if self.machine.gpu.is_some() {
                out.push(Box::new(Onnx::new(file.clone(), Device::Gpu)));
            }
            out.push(Box::new(Onnx::new(file, Device::Cpu)));
        }
        out.push(Box::new(Tract::new(self.machine.threads)));
        out
    }

    /// The model in use for `task`.
    fn chosen(&self, task: VisionTask) -> &'static Model {
        let id = self.state().settings.chosen.get(name(task)).cloned();
        id.and_then(|id| model(&id))
            .filter(|m| m.task() == task && self.store.has(m))
            .unwrap_or_else(|| default_of(task))
    }

    /// The model in use for `task`, ready.
    fn ready(&self, task: VisionTask) -> Result<(&'static Model, Arc<dyn Session>), VisionError> {
        let chosen = self.chosen(task);
        let _one = self.loading.lock().unwrap_or_else(PoisonError::into_inner);
        let kept = {
            let state = self.state();
            let slot = match task {
                VisionTask::Cutout => &state.cutout,
                VisionTask::Upscale => &state.upscale,
            };
            slot.as_ref()
                .filter(|loaded| loaded.id == chosen.id)
                .map(|loaded| Arc::clone(&loaded.session))
        };
        if let Some(session) = kept {
            return Ok((chosen, session));
        }
        let bytes = self.store.read(chosen)?;
        let known = self.state().settings.measured.get(chosen.id).cloned();
        let (session, measured) = fastest(
            &self.runtimes_here(),
            &bytes,
            chosen.shape,
            known.unwrap_or_default(),
        )?;
        let session: Arc<dyn Session> = Arc::from(session);
        let mut state = self.state();
        if state.settings.measured.get(chosen.id) != Some(&measured) {
            state
                .settings
                .measured
                .insert(chosen.id.to_owned(), measured);
            // Only a measure: it is taken again next time.
            let _ = self.store.save(&state.settings);
        }
        let loaded = Some(Loaded {
            id: chosen.id,
            session: Arc::clone(&session),
        });
        match task {
            VisionTask::Cutout => state.cutout = loaded,
            VisionTask::Upscale => state.upscale = loaded,
        }
        Ok((chosen, session))
    }

    /// Forgets what was loaded and measured: there are other runtimes.
    fn start_over(&self) -> Result<(), VisionError> {
        let mut state = self.state();
        state.cutout = None;
        state.upscale = None;
        state.settings.measured.clear();
        self.store.save(&state.settings)
    }
}

/// The bytes of a heavy model.
const HEAVY: usize = 20_000_000;

type Measured = std::collections::BTreeMap<String, u32>;

/// The session of the fastest of `runtimes` for a model, and the ms of each.
/// The ones in `known` are not timed again; one that fails is left out.
fn fastest(
    runtimes: &[Box<dyn Runtime>],
    bytes: &[u8],
    shape: Shape,
    known: Measured,
) -> Result<(Box<dyn Session>, Measured), VisionError> {
    let mut error = VisionError::Unavailable("no runtime".to_owned());
    let all_known = runtimes.iter().all(|r| known.contains_key(r.id()));
    if all_known {
        // Straight to the best one; to the next if it fails now.
        let mut order: Vec<&Box<dyn Runtime>> = runtimes.iter().collect();
        order.sort_by_key(|r| known.get(r.id()).copied().unwrap_or(u32::MAX));
        for runtime in order {
            match runtime.load(bytes, shape.input()) {
                Ok(session) => return Ok((session, known)),
                Err(failed) => error = failed,
            }
        }
        return Err(error);
    }
    let mut measured = Measured::new();
    let mut best: Option<(u32, Box<dyn Session>)> = None;
    for (i, runtime) in runtimes.iter().enumerate() {
        // A heavy model takes the surest one, the last, a long time: it is
        // not worth timing when another runs it.
        if i + 1 == runtimes.len() && bytes.len() > HEAVY && best.is_some() {
            measured.insert(runtime.id().to_owned(), u32::MAX);
            continue;
        }
        match runtime
            .load(bytes, shape.input())
            .and_then(|s| timed(s, shape))
        {
            Ok((ms, session)) => {
                measured.insert(runtime.id().to_owned(), ms);
                if best.as_ref().is_none_or(|(least, _)| ms < *least) {
                    best = Some((ms, session));
                }
            }
            Err(failed) => {
                // Known not to run, so it is not tried each time.
                measured.insert(runtime.id().to_owned(), u32::MAX);
                error = failed;
            }
        }
    }
    best.map(|(_, session)| (session, measured)).ok_or(error)
}

/// How long a piece of work takes `session`, once warm: one among as many as
/// it does at once, for the models that work on pieces.
fn timed(session: Box<dyn Session>, shape: Shape) -> Result<(u32, Box<dyn Session>), VisionError> {
    let input = vec![0.5_f32; shape.input().iter().product()];
    let start = Instant::now();
    session.run(input.clone())?;
    let first = start.elapsed();
    let at_once = match shape {
        Shape::Upscale { .. } => session.at_once(),
        Shape::Cutout { .. } => 1,
    };
    // The first one prepares the graphics card; a slow one is slow.
    let warm = if at_once > 1 {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(at_once)
            .build()
            .map_err(|e| VisionError::Model(e.to_string()))?;
        let start = Instant::now();
        pool.install(|| {
            (0..at_once)
                .into_par_iter()
                .try_for_each(|_| session.run(input.clone()).map(drop))
        })?;
        // Few at once: the cast is exact.
        start.elapsed() / at_once as u32
    } else if first.as_millis() > 3000 {
        first
    } else {
        let start = Instant::now();
        session.run(input)?;
        start.elapsed()
    };
    let ms = u32::try_from(warm.as_millis()).unwrap_or(u32::MAX - 1);
    Ok((ms.max(1), session))
}

impl VisionModels for Vision {
    fn machine(&self) -> Machine {
        self.machine.clone()
    }

    fn models(&self) -> Vec<ModelInfo> {
        let settings = self.state().settings.clone();
        MODELS
            .iter()
            .map(|m| {
                let task = m.task();
                let measured = settings.measured.get(m.id).and_then(|by| {
                    let runtimes = ["GPU", "CPU+", "CPU"];
                    runtimes
                        .into_iter()
                        .filter_map(|id| Some((*by.get(id)?, id)))
                        .filter(|(ms, _)| *ms != u32::MAX)
                        .min()
                });
                ModelInfo {
                    id: m.id,
                    task: Some(task),
                    name: m.name,
                    about: m.about,
                    license: m.license,
                    source: m.source,
                    download_bytes: if m.bundled.is_some() { 0 } else { m.bytes },
                    bundled: m.bundled.is_some(),
                    installed: self.store.has(m),
                    chosen: self.chosen(task).id == m.id,
                    recommended: right_for(task, &self.machine).id == m.id,
                    measured,
                }
            })
            .collect()
    }

    fn runtimes(&self) -> Vec<ModelInfo> {
        if !cfg!(feature = "onnx") {
            return Vec::new();
        }
        let of = |library: &crate::catalog::Library| ModelInfo {
            id: library.id,
            task: None,
            name: library.name,
            about: library.about,
            license: library.license,
            source: library.source,
            download_bytes: library.bytes,
            bundled: false,
            installed: self.store.library_file(library).is_some(),
            chosen: self.store.library_file(library).is_some(),
            recommended: library_is_for(&self.machine),
            measured: None,
        };
        ONNX_RUNTIME.iter().map(of).collect()
    }

    fn install(
        &self,
        id: &str,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<(), VisionError> {
        if let Some(library) = ONNX_RUNTIME.as_ref().filter(|library| library.id == id) {
            self.store
                .install_library(&*self.fetch, library, progress, cancel)?;
            return self.start_over();
        }
        let model = model(id).ok_or_else(|| VisionError::Unavailable(id.to_owned()))?;
        self.store
            .install_model(&*self.fetch, model, progress, cancel)
    }

    fn uninstall(&self, id: &str) -> Result<(), VisionError> {
        if let Some(library) = ONNX_RUNTIME.as_ref().filter(|library| library.id == id) {
            // What is loaded of it goes on until Gafira closes.
            self.store.remove_library(library)?;
            return self.start_over();
        }
        let model = model(id).ok_or_else(|| VisionError::Unavailable(id.to_owned()))?;
        self.store.remove_model(model)
    }

    fn choose(&self, id: &str) -> Result<(), VisionError> {
        let unavailable = || VisionError::Unavailable(id.to_owned());
        let model = model(id).ok_or_else(unavailable)?;
        if !self.store.has(model) {
            return Err(unavailable());
        }
        let mut state = self.state();
        let task = name(model.task()).to_owned();
        state.settings.chosen.insert(task, id.to_owned());
        self.store.save(&state.settings)
    }

    fn remover(&self) -> Result<Arc<dyn BackgroundRemover>, VisionError> {
        let (model, session) = self.ready(VisionTask::Cutout)?;
        let Shape::Cutout {
            side,
            mean,
            deviation,
        } = model.shape
        else {
            return Err(VisionError::Unavailable(model.id.to_owned()));
        };
        Ok(Arc::new(Cutter {
            session,
            side,
            mean,
            deviation,
        }))
    }

    fn upscaler(&self) -> Result<Arc<dyn Upscaler>, VisionError> {
        let (model, session) = self.ready(VisionTask::Upscale)?;
        let Shape::Upscale { tile, scale } = model.shape else {
            return Err(VisionError::Unavailable(model.id.to_owned()));
        };
        Ok(Arc::new(Enlarger {
            session,
            tile,
            scale,
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use gafira_doc::contract::NoProgress;

    use super::*;
    use crate::fetch::NoNetwork;

    /// A runtime that takes some ms, or fails, and counts its loads.
    struct Fake(&'static str, Option<u64>, Arc<AtomicUsize>);

    struct Takes(u64);

    impl Session for Takes {
        fn run(&self, input: Vec<f32>) -> Result<Vec<f32>, VisionError> {
            std::thread::sleep(Duration::from_millis(self.0));
            Ok(input)
        }
    }

    impl Runtime for Fake {
        fn id(&self) -> &'static str {
            self.0
        }

        fn load(&self, _: &[u8], _: [usize; 4]) -> Result<Box<dyn Session>, VisionError> {
            self.2.fetch_add(1, Ordering::Relaxed);
            let ms = self.1.ok_or(VisionError::Model("no".into()))?;
            Ok(Box::new(Takes(ms)))
        }
    }

    fn fakes(list: &[(&'static str, Option<u64>)]) -> (Vec<Box<dyn Runtime>>, Arc<AtomicUsize>) {
        let loads = Arc::new(AtomicUsize::new(0));
        let make = |(id, ms): &(&'static str, Option<u64>)| {
            Box::new(Fake(id, *ms, Arc::clone(&loads))) as Box<dyn Runtime>
        };
        (list.iter().map(make).collect(), loads)
    }

    // S84-04: the fastest is used, one that fails is not, and what is known
    // is not timed again.
    #[test]
    fn s84_04_the_fastest_runtime() {
        let shape = Shape::Upscale { tile: 1, scale: 1 };
        let (all, loads) = fakes(&[("GPU", None), ("CPU+", Some(2)), ("CPU", Some(60))]);
        let (_, measured) = fastest(&all, &[], shape, Measured::new()).unwrap();
        assert_eq!(measured["GPU"], u32::MAX);
        assert!(measured["CPU+"] < measured["CPU"]);
        assert_eq!(loads.swap(0, Ordering::Relaxed), 3);
        let (_, again) = fastest(&all, &[], shape, measured.clone()).unwrap();
        assert_eq!((again, loads.load(Ordering::Relaxed)), (measured, 1));
        // The best one fails now: the next.
        let known: Measured = [("GPU".to_owned(), 1), ("CPU".to_owned(), 9)].into();
        let (two, _) = fakes(&[("GPU", None), ("CPU", Some(1))]);
        assert!(fastest(&two, &[], shape, known).is_ok());
        let (none, _) = fakes(&[("GPU", None)]);
        assert!(fastest(&none, &[], shape, Measured::new()).is_err());
        assert!(fastest(&[], &[], shape, Measured::new()).is_err());
    }

    fn vision(name: &str) -> (Vision, PathBuf) {
        let folder =
            std::env::temp_dir().join(format!("gafira-service-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let machine = Machine {
            threads: 8,
            ..Machine::default()
        };
        (
            Vision::new(folder.clone(), Box::new(NoNetwork), machine),
            folder,
        )
    }

    // S84-05, S84-09: without network, the models that come with Gafira are
    // there and in use; another cannot be chosen until it is installed.
    #[test]
    fn s84_09_the_bundled_models_are_in_use() {
        let (vision, folder) = vision("bundled");
        let all = vision.models();
        let chosen: Vec<&str> = all.iter().filter(|m| m.chosen).map(|m| m.id).collect();
        assert_eq!(chosen, ["u2netp", "realesr-general-x4v3"]);
        assert!(all.iter().all(|m| m.installed == m.bundled));
        assert!(vision.choose("u2net").is_err() && vision.choose("what").is_err());
        let out = vision.install("u2net", &NoProgress, &CancelToken::new());
        assert_eq!(out, Err(VisionError::Offline));
        assert_eq!(vision.choose("u2netp"), Ok(()));
        let _ = std::fs::remove_dir_all(folder);
    }

    // S84-03, S84-10: the real models that come with Gafira find a subject
    // and make an image larger.
    #[test]
    fn s84_03_the_bundled_models_work() {
        use gafira_ports::vision::Pixels;
        let (vision, folder) = vision("works");
        // A light disc on a dark ground.
        let (w, h) = (96_u32, 64_u32);
        let bytes = (0..w * h).flat_map(|i| {
            let (x, y) = ((i % w) as f32 - 48.0, (i / w) as f32 - 32.0);
            if x * x + y * y < 400.0 {
                [240, 190, 60, 255]
            } else {
                [20, 30, 60, 255]
            }
        });
        let image = Pixels::new(w, h, bytes.collect()).unwrap();
        let cancel = CancelToken::new();
        let mask = vision
            .remover()
            .unwrap()
            .mask(&image, &NoProgress, &cancel)
            .unwrap();
        let at = |x: u32, y: u32| mask.data()[(y * w + x) as usize];
        assert!(
            at(48, 32) > 200 && at(3, 3) < 60 && at(92, 60) < 60,
            "{} {}",
            at(48, 32),
            at(3, 3)
        );
        let larger = vision
            .upscaler()
            .unwrap()
            .upscale(&image, 2, &NoProgress, &cancel)
            .unwrap();
        assert_eq!(larger.size(), (w * 2, h * 2));
        let p = |x: usize, y: usize| larger.rgba()[(y * 192 + x) * 4];
        assert!(p(96, 64) > 200 && p(4, 4) < 60);
        assert!(
            vision
                .models()
                .iter()
                .filter(|m| m.bundled)
                .all(|m| m.measured.is_some())
        );
        let _ = std::fs::remove_dir_all(folder);
    }
}
