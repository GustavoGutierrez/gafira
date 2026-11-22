//! `tract`: models in pure Rust, on the processor, always there.

use std::sync::Arc;

use gafira_ports::vision::VisionError;
use rayon::ThreadPool;
use tract_linalg::multithread::{Executor, multithread_tract_scope};
use tract_onnx::prelude::*;

use super::{Input, Runtime, Session, as_bytes};

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

    fn load(&self, bytes: &[u8], inputs: &[Input]) -> Result<Box<dyn Session>, VisionError> {
        let mut model = tract_onnx::onnx()
            .model_for_read(&mut std::io::Cursor::new(bytes))
            .map_err(failed)?;
        for (at, input) in inputs.iter().enumerate() {
            let fact = if input.bytes {
                u8::fact(input.shape)
            } else {
                f32::fact(input.shape)
            };
            model = model.with_input_fact(at, fact.into()).map_err(failed)?;
        }
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
            inputs: inputs.to_vec(),
            pool: Arc::new(pool),
            threads: self.threads,
        }))
    }
}

struct Ready {
    model: Arc<TypedRunnableModel>,
    inputs: Vec<Input>,
    pool: Arc<ThreadPool>,
    threads: usize,
}

impl Session for Ready {
    fn run(&self, inputs: Vec<Vec<f32>>) -> Result<Vec<f32>, VisionError> {
        let given: TVec<TValue> = self
            .inputs
            .iter()
            .zip(inputs)
            .map(|(input, values)| {
                let tensor = if input.bytes {
                    Tensor::from_shape(&input.shape, &as_bytes(values))
                } else {
                    Tensor::from_shape(&input.shape, &values)
                };
                tensor.map(Into::into).map_err(failed)
            })
            .collect::<Result<_, _>>()?;
        let run = || self.model.run(given);
        // A piece among many takes one thread; a model alone, all of them.
        let out = if rayon::current_thread_index().is_some() {
            run()
        } else {
            multithread_tract_scope(Executor::MultiThread(Arc::clone(&self.pool)), run)
        };
        let out = out.map_err(failed)?;
        let first = out.first().ok_or_else(|| failed("no output"))?;
        if first.datum_type() == u8::datum_type() {
            let view = first.to_plain_array_view::<u8>().map_err(failed)?;
            return Ok(view.iter().copied().map(f32::from).collect());
        }
        let view = first.to_plain_array_view::<f32>().map_err(failed)?;
        Ok(view.iter().copied().collect())
    }

    fn at_once(&self) -> usize {
        self.threads
    }
}
