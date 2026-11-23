//! ONNX Runtime, from the library the person installed: on the graphics
//! card, or on the processor.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

use gafira_ports::vision::VisionError;
use ort::environment::Environment;
use ort::session::Session as OrtSession;
use ort::session::SessionInputValue;
use ort::value::Tensor;

use super::{Input, Runtime, Session, as_bytes};

/// What it runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Gpu,
    Cpu,
}

pub struct Onnx {
    library: PathBuf,
    device: Device,
}

impl Onnx {
    #[must_use]
    pub fn new(library: PathBuf, device: Device) -> Self {
        Self { library, device }
    }
}

/// The library, loaded once for the life of the program.
fn environment(library: &Path) -> Result<&'static Environment, VisionError> {
    static LOADED: OnceLock<Result<Environment, String>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            ort::init_from(library)
                .map_err(|e| e.to_string())?
                .with_telemetry(false)
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|error| VisionError::Unavailable(error.clone()))
}

fn failed(error: impl std::fmt::Display) -> VisionError {
    VisionError::Model(error.to_string())
}

impl Runtime for Onnx {
    fn id(&self) -> &'static str {
        match self.device {
            Device::Gpu => "GPU",
            Device::Cpu => "CPU+",
        }
    }

    fn load(&self, bytes: &[u8], inputs: &[Input]) -> Result<Box<dyn Session>, VisionError> {
        let environment = environment(&self.library)?;
        let mut builder = OrtSession::builder(environment).map_err(failed)?;
        if self.device == Device::Gpu {
            let gpu = ort::ep::WebGPU::default().build().error_on_failure();
            builder = builder.with_execution_providers([gpu]).map_err(failed)?;
        }
        let session = builder.commit_from_memory(bytes).map_err(failed)?;
        let names: Vec<String> = session
            .inputs()
            .iter()
            .map(|input| input.name().to_owned())
            .collect();
        if names.len() < inputs.len() {
            return Err(failed("not the inputs there should be"));
        }
        let inputs = names.into_iter().zip(inputs.iter().copied()).collect();
        Ok(Box::new(Ready {
            session: Mutex::new(session),
            inputs,
        }))
    }
}

struct Ready {
    session: Mutex<OrtSession>,
    inputs: Vec<(String, Input)>,
}

impl Session for Ready {
    fn run(&self, inputs: Vec<Vec<f32>>) -> Result<Vec<f32>, VisionError> {
        let mut given: Vec<(&str, SessionInputValue<'_>)> = Vec::new();
        for ((name, input), values) in self.inputs.iter().zip(inputs) {
            let value: SessionInputValue<'_> = if input.bytes {
                let tensor = Tensor::from_array((input.shape, as_bytes(values)));
                tensor.map_err(failed)?.into()
            } else {
                Tensor::from_array((input.shape, values))
                    .map_err(failed)?
                    .into()
            };
            given.push((name.as_str(), value));
        }
        let mut session = self.session.lock().unwrap_or_else(PoisonError::into_inner);
        let out = session.run(given).map_err(failed)?;
        if let Ok((_, values)) = out[0].try_extract_tensor::<u8>() {
            return Ok(values.iter().copied().map(f32::from).collect());
        }
        let (_, values) = out[0].try_extract_tensor::<f32>().map_err(failed)?;
        Ok(values.to_vec())
    }
}
