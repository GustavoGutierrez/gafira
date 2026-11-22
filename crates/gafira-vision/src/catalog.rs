//! The models and the runtimes there are (S84-03, S84-05): what each is,
//! where it comes from and how its image is given to it.

use gafira_ports::vision::{Machine, VisionTask};

use crate::runtime::Input;

/// How a model takes its image and what it gives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// The image of `side` × `side`, less `mean` and over `deviation`; its
    /// opacity comes out.
    Cutout {
        side: usize,
        mean: [f32; 3],
        deviation: [f32; 3],
    },
    /// Pieces of `tile` × `tile`, 0 to 1; they come out `scale` times larger.
    Upscale { tile: usize, scale: u32 },
    /// Pieces of `side` × `side` and what to fill of them.
    Inpaint { side: usize, kind: Filling },
}

/// How a model that fills takes its piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filling {
    /// Bytes; the mask is 255 on what is kept.
    Bytes,
    /// Numbers from 0 to 1; the mask is 1 on what is filled.
    Numbers,
}

impl Shape {
    /// What goes in.
    #[must_use]
    pub fn inputs(self) -> Vec<Input> {
        match self {
            Self::Cutout { side, .. } => vec![Input::numbers([1, 3, side, side])],
            Self::Upscale { tile, .. } => vec![Input::numbers([1, 3, tile, tile])],
            Self::Inpaint { side, kind } => [3, 1]
                .map(|channels| Input {
                    shape: [1, channels, side, side],
                    bytes: kind == Filling::Bytes,
                })
                .to_vec(),
        }
    }
}

/// A model.
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    pub license: &'static str,
    pub source: &'static str,
    pub bytes: u64,
    /// BLAKE3 of the file.
    pub hash: &'static str,
    pub url: &'static str,
    /// Its file, when it comes with Gafira.
    pub bundled: Option<&'static [u8]>,
    pub shape: Shape,
}

impl Model {
    #[must_use]
    pub fn task(&self) -> VisionTask {
        match self.shape {
            Shape::Cutout { .. } => VisionTask::Cutout,
            Shape::Upscale { .. } => VisionTask::Upscale,
            Shape::Inpaint { .. } => VisionTask::Inpaint,
        }
    }
}

/// The mean and the deviation of the photos U²-Net was made with.
const PHOTOS: ([f32; 3], [f32; 3]) = ([0.485, 0.456, 0.406], [0.229, 0.224, 0.225]);

/// The models, by task, the lightest first.
pub const MODELS: [Model; 8] = [
    Model {
        id: "u2netp",
        name: "U²-Net P",
        about: "vision.about_u2netp",
        license: "Apache-2.0",
        source: "github.com/xuebinqin/U-2-Net",
        bytes: 4_574_861,
        hash: "8b4bde6560ee1edb10b48184ecad7895a61aa8ebfd7c651089a7990172eb7b74",
        url: "",
        bundled: Some(include_bytes!("../../../assets/models/u2netp.onnx")),
        shape: Shape::Cutout {
            side: 320,
            mean: PHOTOS.0,
            deviation: PHOTOS.1,
        },
    },
    Model {
        id: "u2net",
        name: "U²-Net",
        about: "vision.about_u2net",
        license: "Apache-2.0",
        source: "github.com/xuebinqin/U-2-Net",
        bytes: 175_997_641,
        hash: "04be16bb01712f04397fd382522571a9ff88d45b1509f37b636dbfff227403ad",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
        bundled: None,
        shape: Shape::Cutout {
            side: 320,
            mean: PHOTOS.0,
            deviation: PHOTOS.1,
        },
    },
    Model {
        id: "u2net-human",
        name: "U²-Net Human",
        about: "vision.about_u2net_human",
        license: "Apache-2.0",
        source: "github.com/xuebinqin/U-2-Net",
        bytes: 175_997_641,
        hash: "71a0354b1cfafe7241c9d46b08cea3af8904818f8ac4c649317e24945f4bbb83",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net_human_seg.onnx",
        bundled: None,
        shape: Shape::Cutout {
            side: 320,
            mean: PHOTOS.0,
            deviation: PHOTOS.1,
        },
    },
    Model {
        id: "isnet-general",
        name: "IS-Net",
        about: "vision.about_isnet",
        license: "Apache-2.0",
        source: "github.com/xuebinqin/DIS",
        bytes: 178_648_008,
        hash: "a4803d041160d956c81853125e6391608a3e9b7772e37bcbcc86aa3d87d6b214",
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx",
        bundled: None,
        shape: Shape::Cutout {
            side: 1024,
            mean: [0.5; 3],
            deviation: [1.0; 3],
        },
    },
    Model {
        id: "realesr-general-x4v3",
        name: "Real-ESRGAN General x4v3",
        about: "vision.about_general",
        license: "BSD-3-Clause",
        source: "github.com/xinntao/Real-ESRGAN",
        bytes: 4_866_386,
        hash: "6ed5c33f7c61bcf24898c0e417ecfa6215755eada992a9d11bc23120d636e067",
        url: "",
        bundled: Some(include_bytes!(
            "../../../assets/models/realesr-general-x4v3.onnx"
        )),
        shape: Shape::Upscale {
            tile: 128,
            scale: 4,
        },
    },
    Model {
        id: "realesrgan-x4plus",
        name: "Real-ESRGAN x4plus",
        about: "vision.about_x4plus",
        license: "BSD-3-Clause",
        source: "github.com/xinntao/Real-ESRGAN",
        bytes: 67_160_311,
        hash: "a642880316dde057585501a05cfc3a24c20cf5aba703f4a6f3e0729dcf50b54f",
        url: "https://huggingface.co/bukuroo/RealESRGAN-ONNX/resolve/a1f365e2ef85ca5d4c66325e76bc7e26374c9642/real-esrgan-x4plus-128.onnx",
        bundled: None,
        shape: Shape::Upscale {
            tile: 128,
            scale: 4,
        },
    },
    Model {
        id: "migan",
        name: "MI-GAN",
        about: "vision.about_migan",
        license: "MIT",
        source: "github.com/Picsart-AI-Research/MI-GAN",
        bytes: 29_546_882,
        hash: "0666f41bb92884ff439acaca5a72d2cbc197b770f1c465c44dab8fc0183d51b5",
        url: "https://huggingface.co/andraniksargsyan/migan/resolve/406830d0fa60666da0071c342ad2fbc8f30c5c64/migan.onnx",
        bundled: None,
        shape: Shape::Inpaint {
            side: 512,
            kind: Filling::Bytes,
        },
    },
    Model {
        id: "lama",
        name: "LaMa",
        about: "vision.about_lama",
        license: "Apache-2.0",
        source: "github.com/advimman/lama",
        bytes: 208_044_816,
        hash: "ce4b55cc81ddacf2aaf0bbf676d0a497c8cd30aac7b4c85d85227a77c2b52d3d",
        url: "https://huggingface.co/Carve/LaMa-ONNX/resolve/c3c0c9e468934d62e79c329e35d82dd09ff8c444/lama_fp32.onnx",
        bundled: None,
        shape: Shape::Inpaint {
            side: 512,
            kind: Filling::Numbers,
        },
    },
];

/// A library that runs models, in a ZIP.
#[derive(Clone, Copy, Debug)]
pub struct Library {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    pub license: &'static str,
    pub source: &'static str,
    pub bytes: u64,
    pub hash: &'static str,
    pub url: &'static str,
    /// The files taken from the ZIP and how they are kept, the library
    /// itself first.
    pub files: &'static [(&'static str, &'static str)],
}

/// ONNX Runtime with WebGPU (Vulkan, Direct3D or Metal), for this system.
pub const ONNX_RUNTIME: Option<Library> = library();

const fn onnx(
    bytes: u64,
    hash: &'static str,
    url: &'static str,
    files: &'static [(&'static str, &'static str)],
) -> Option<Library> {
    Some(Library {
        id: "onnxruntime",
        name: "ONNX Runtime 1.27",
        about: "vision.about_onnxruntime",
        license: "MIT",
        source: "github.com/microsoft/onnxruntime",
        bytes,
        hash,
        url,
        files,
    })
}

const fn library() -> Option<Library> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return onnx(
            25_163_729,
            "d96c9e3008496e74132d0c2b2e972d631b7c436f556ad674711cdb0a00f7f4d8",
            "https://files.pythonhosted.org/packages/24/11/6cd5f840be97819cbd172bacc1868726ab5fa54ceaaa79508c5a64e61233/onnxruntime_webgpu-1.27.0-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl",
            &[(
                "onnxruntime/capi/libonnxruntime.so.1.27.0",
                "libonnxruntime.so",
            )],
        );
    }
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return onnx(
            27_226_899,
            "184e01ca1d4f3b1ba904c833403d2eb1e999d7dec8fb164af0bb681f00b3b602",
            "https://files.pythonhosted.org/packages/df/28/016260c51c877ba5b3eba823b43107e894659e73e0ecf250eb07e801e3c2/onnxruntime_webgpu-1.27.0-cp312-cp312-win_amd64.whl",
            &[
                ("onnxruntime/capi/onnxruntime.dll", "onnxruntime.dll"),
                ("onnxruntime/capi/dxcompiler.dll", "dxcompiler.dll"),
                ("onnxruntime/capi/dxil.dll", "dxil.dll"),
            ],
        );
    }
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return onnx(
            24_081_620,
            "a1a362217ad7d415f5dc450192956c7ea6f1a8eb83171cf5937a3e8998f6d685",
            "https://files.pythonhosted.org/packages/b4/1f/be670672fa0dc367fd28113b5e410a6e71cb6ee66c188ed1a8eb1439c941/onnxruntime_webgpu-1.27.0-cp312-cp312-macosx_14_0_arm64.whl",
            &[(
                "onnxruntime/capi/libonnxruntime.1.27.0.dylib",
                "libonnxruntime.dylib",
            )],
        );
    }
    None
}

/// The model `id`.
#[must_use]
pub fn model(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.id == id)
}

/// The model a task starts with: the one that comes with Gafira, or the
/// lightest.
#[must_use]
pub fn default_of(task: VisionTask) -> &'static Model {
    MODELS
        .iter()
        .find(|m| m.task() == task && m.bundled.is_some())
        .or_else(|| MODELS.iter().find(|m| m.task() == task))
        .unwrap_or(&MODELS[0])
}

/// A gigabyte, about.
const GB: u64 = 1_000_000_000;

/// Whether the library that uses the graphics card is right for `machine`.
#[must_use]
pub fn library_is_for(machine: &Machine) -> bool {
    ONNX_RUNTIME.is_some() && machine.gpu.as_ref().is_some_and(|g| g.dedicated)
}

/// The model of `task` that is right for `machine`: the best one it runs in
/// a moment, the lightest when it has few resources.
#[must_use]
pub fn right_for(task: VisionTask, machine: &Machine) -> &'static Model {
    let dedicated = machine.gpu.as_ref().is_some_and(|g| g.dedicated);
    // Not known is not little.
    let room = machine.memory_bytes == 0 || machine.memory_bytes >= 8 * GB;
    let able = machine.threads >= 8 && room;
    let id = match task {
        VisionTask::Cutout if dedicated && room => "isnet-general",
        VisionTask::Cutout if able => "u2net",
        VisionTask::Upscale if dedicated && room => "realesrgan-x4plus",
        VisionTask::Inpaint if able || (dedicated && room) => "lama",
        _ => return default_of(task),
    };
    model(id).unwrap_or_else(|| default_of(task))
}

#[cfg(test)]
mod tests {
    use gafira_ports::vision::Gpu;

    use super::*;

    // S84-05, S84-06: every model says where it comes from and has its file
    // or the way to it; the ones that come with Gafira are what they say.
    #[test]
    fn s84_06_the_catalogue_is_whole() {
        for (i, m) in MODELS.iter().enumerate() {
            assert!(MODELS[..i].iter().all(|o| o.id != m.id), "{}", m.id);
            assert_eq!(m.hash.len(), 64);
            assert!(
                ["Apache-2.0", "BSD-3-Clause", "MIT"].contains(&m.license),
                "{}",
                m.id
            );
            match m.bundled {
                Some(bytes) => {
                    assert_eq!(bytes.len() as u64, m.bytes);
                    assert_eq!(blake3::hash(bytes).to_hex().as_str(), m.hash, "{}", m.id);
                }
                None => assert!(m.url.starts_with("https://"), "{}", m.id),
            }
        }
        // D-115: ten megabytes in all come with Gafira.
        let bundled: u64 = MODELS
            .iter()
            .filter(|m| m.bundled.is_some())
            .map(|m| m.bytes)
            .sum();
        assert!(bundled < 10_000_000);
        for task in [VisionTask::Cutout, VisionTask::Upscale, VisionTask::Inpaint] {
            assert_eq!(default_of(task).task(), task);
        }
        if let Some(library) = ONNX_RUNTIME {
            assert!(library.url.starts_with("https://") && !library.files.is_empty());
        }
    }

    // S86-03: what fills is never bundled; the right one for a machine.
    #[test]
    fn s86_03_what_fills() {
        let of = |threads, memory_bytes| {
            let machine = Machine {
                threads,
                memory_bytes,
                gpu: None,
            };
            right_for(VisionTask::Inpaint, &machine).id
        };
        assert_eq!(default_of(VisionTask::Inpaint).id, "migan");
        assert_eq!(
            (of(4, 16 * GB), of(8, 16 * GB), of(16, 4 * GB)),
            ("migan", "lama", "migan")
        );
        for id in ["migan", "lama"] {
            let inputs = model(id).expect(id).shape.inputs();
            let shapes: Vec<usize> = inputs.iter().map(|i| i.shape[1]).collect();
            assert_eq!(shapes, [3, 1], "{id}");
        }
    }

    // S84-05: what is right for a machine.
    #[test]
    fn s84_05_the_right_model() {
        let of = |threads, gb: u64, dedicated: Option<bool>| {
            let machine = Machine {
                threads,
                memory_bytes: gb * GB,
                gpu: dedicated.map(|dedicated| Gpu {
                    name: "x".into(),
                    dedicated,
                }),
            };
            let id = |task| right_for(task, &machine).id;
            (id(VisionTask::Cutout), id(VisionTask::Upscale))
        };
        assert_eq!(of(4, 8, None), ("u2netp", "realesr-general-x4v3"));
        assert_eq!(of(16, 4, None), ("u2netp", "realesr-general-x4v3"));
        assert_eq!(of(16, 16, Some(false)), ("u2net", "realesr-general-x4v3"));
        assert_eq!(of(8, 0, None), ("u2net", "realesr-general-x4v3"));
        assert_eq!(
            of(16, 16, Some(true)),
            ("isnet-general", "realesrgan-x4plus")
        );
    }
}
