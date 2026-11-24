//! The icon and the details of the Windows executable (spec 090), and
//! the templates the app carries inside (spec 091).

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// Every `.gaf` under `dir`, by path.
fn gafs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gafs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "gaf") {
            out.push(path);
        }
    }
}

/// `$OUT_DIR/templates.rs`: `BUNDLED`, each template of
/// `assets/templates/` as its path under that folder and its bytes.
fn templates() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("the manifest dir");
    let root = Path::new(&manifest).join("../../assets/templates");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut found = Vec::new();
    gafs(&root, &mut found);
    found.sort();
    let mut code = String::from("pub(crate) const BUNDLED: &[(&str, &[u8])] = &[\n");
    for path in &found {
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path
            .strip_prefix(&root)
            .expect("under the folder")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        // CARGO_MANIFEST_DIR is absolute, so the path is too.
        code.push_str(&format!(
            "    ({relative:?}, include_bytes!({:?})),\n",
            path.display().to_string()
        ));
    }
    code.push_str("];\n");
    let out = std::env::var("OUT_DIR").expect("the out dir");
    std::fs::write(Path::new(&out).join("templates.rs"), code).expect("writes templates.rs");
}

fn main() {
    templates();
    const ICON: &str = "../../assets/app-icon/windows/gafira.ico";
    println!("cargo:rerun-if-changed={ICON}");
    println!("cargo:rerun-if-changed=build.rs");
    // The system it is built for, not the one that builds it.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(ICON)
        .set("ProductName", "Gafira")
        .set("FileDescription", "Gafira")
        .set("CompanyName", "Gustavo Gutiérrez")
        .set("OriginalFilename", "Gafira.exe")
        .set("LegalCopyright", "Copyright (c) 2026 Gustavo Gutiérrez");
    resource.compile().expect("the resources of the executable");
}
