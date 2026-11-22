//! The icon and the details of the Windows executable (spec 090).

#![forbid(unsafe_code)]

fn main() {
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
