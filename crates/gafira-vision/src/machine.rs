//! What the models run on (S84-05).

use std::fs;
use std::path::Path;

use gafira_ports::vision::{Gpu, Machine};

/// This machine.
#[must_use]
pub fn detect() -> Machine {
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        // Its graphics share the memory and are fast all the same.
        return Machine {
            threads,
            memory_bytes: 0,
            gpu: Some(Gpu {
                name: "Apple".to_owned(),
                dedicated: true,
            }),
        };
    }
    from_files(Path::new("/"), threads)
}

/// The machine the files of Linux under `root` tell of; what they do not is
/// not known.
fn from_files(root: &Path, threads: usize) -> Machine {
    let read = |path: &str| fs::read_to_string(root.join(path)).unwrap_or_default();
    let memory_bytes = read("proc/meminfo")
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))
        .and_then(|rest| rest.split_whitespace().next()?.parse::<u64>().ok())
        .map_or(0, |kb| kb * 1024);
    let mut gpus = Vec::new();
    let mut cards: Vec<_> = fs::read_dir(root.join("sys/class/drm"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            // `card0`, not its outputs, `card0-DP-1`.
            name.strip_prefix("card")
                .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(|entry| entry.path().join("device"))
        .collect();
    cards.sort();
    for device in cards {
        let file = |name: &str| fs::read_to_string(device.join(name)).unwrap_or_default();
        match file("vendor").trim() {
            "0x10de" => gpus.push(Gpu {
                name: nvidia(root).unwrap_or_else(|| "NVIDIA".to_owned()),
                dedicated: true,
            }),
            "0x1002" => {
                let vram: u64 = file("mem_info_vram_total").trim().parse().unwrap_or(0);
                gpus.push(Gpu {
                    name: "AMD Radeon".to_owned(),
                    dedicated: vram >= 2 << 30,
                });
            }
            "0x8086" => gpus.push(Gpu {
                name: "Intel".to_owned(),
                dedicated: false,
            }),
            _ => {}
        }
    }
    gpus.sort_by_key(|gpu| !gpu.dedicated);
    Machine {
        threads,
        memory_bytes,
        gpu: gpus.into_iter().next(),
    }
}

/// The name its driver gives the card.
fn nvidia(root: &Path) -> Option<String> {
    let cards = fs::read_dir(root.join("proc/driver/nvidia/gpus")).ok()?;
    cards.flatten().find_map(|card| {
        let text = fs::read_to_string(card.path().join("information")).ok()?;
        let model = text.lines().find_map(|l| l.strip_prefix("Model:"))?.trim();
        (!model.is_empty()).then(|| model.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // S84-05: the memory, and the best card: a dedicated one before the
    // processor's; an unknown system tells nothing.
    #[test]
    fn s84_05_the_machine() {
        let root = std::env::temp_dir().join(format!("gafira-machine-{}", std::process::id()));
        let put = |path: &str, text: &str| {
            let path = root.join(path);
            fs::create_dir_all(path.parent().expect("a folder")).expect("a folder");
            fs::write(path, text).expect("a file");
        };
        assert_eq!(from_files(&root, 4).gpu, None);
        assert_eq!(from_files(&root, 4).memory_bytes, 0);
        put(
            "proc/meminfo",
            "MemFree: 1 kB\nMemTotal:       16000000 kB\n",
        );
        put("sys/class/drm/card0/device/vendor", "0x8086\n");
        put("sys/class/drm/card0-DP-1/device/vendor", "0x10de\n");
        let machine = from_files(&root, 4);
        assert_eq!(machine.memory_bytes, 16_384_000_000);
        assert_eq!(machine.gpu.map(|g| g.dedicated), Some(false));
        put("sys/class/drm/card1/device/vendor", "0x10de\n");
        put(
            "proc/driver/nvidia/gpus/0/information",
            "Model: \t\t NVIDIA GeForce\nIRQ: 1\n",
        );
        let gpu = from_files(&root, 4).gpu.expect("a card");
        assert_eq!((gpu.name.as_str(), gpu.dedicated), ("NVIDIA GeForce", true));
        fs::remove_dir_all(root).expect("clean up");
    }
}
