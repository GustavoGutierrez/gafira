//! `tract`: models in pure Rust, on the processor, always there.

use std::sync::Arc;

use gafira_ports::vision::VisionError;
use rayon::ThreadPool;
use tract_linalg::multithread::{Executor, multithread_tract_scope};
use tract_onnx::prelude::*;

use super::{Runtime, Session};

pub struct Tract {
    /// The threads of one model, or the pieces at once.
    threads: usize,
}

impl Tract {
    /// On a machine of `threads`: half of them, as two share a core.
    #[must_use]
    pub fn new(threads: usize) -> Self {
        Self {
            threads: (threads / 2).clamp(1, 16),
        }
    }
}

fn failed(error: impl std::fmt::Display) -> VisionError {
    VisionError::Model(format!("{error:#}"))
}

impl Runtime for Tract {
    fn id(&self) -> &'static str {
        "CPU"
    }

    fn load(&self, bytes: &[u8], shape: [usize; 4]) -> Result<Box<dyn Session>, VisionError> {
        let mut model = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(bytes))
            .and_then(|model| model.with_input_fact(0, f32::fact(shape).into()))
            .map_err(failed)?;
        // The other outputs are for training.
        model.outputs.truncate(1);
        let model = model
            .into_optimized()
            .and_then(|model| model.into_runnable())
            .map_err(failed)?;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.threads)
            .build()
            .map_err(failed)?;
        Ok(Box::new(Ready {
            model,
            shape,
            pool: Arc::new(pool),
            threads: self.threads,
        }))
    }
}

struct Ready {
    model: Arc<TypedRunnableModel>,
    shape: [usize; 4],
    pool: Arc<ThreadPool>,
    threads: usize,
}

impl Session for Ready {
    fn run(&self, input: Vec<f32>) -> Result<Vec<f32>, VisionError> {
        let tensor = Tensor::from_shape(&self.shape, &input).map_err(failed)?;
        let run = || self.model.run(tvec!(tensor.into()));
        // A piece among many takes one thread; a model alone, all of them.
        let out = if rayon::current_thread_index().is_some() {
            run()
        } else {
            multithread_tract_scope(Executor::MultiThread(Arc::clone(&self.pool)), run)
        };
        let out = out.map_err(failed)?;
        let first = out.first().ok_or_else(|| failed("no output"))?;
        Ok(first
            .to_plain_array_view::<f32>()
            .map_err(failed)?
            .iter()
            .copied()
            .collect())
    }

    fn at_once(&self) -> usize {
        self.threads
    }
}
