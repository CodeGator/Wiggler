fn main() {
    println!("cargo:rerun-if-changed=assets/wiggler.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/wiggler.ico");
    res.set("ProductName", "Wiggler");
    res.set("FileDescription", "Nudge the mouse so Windows stays active.");
    res.set("CompanyName", "CodeGator");
    res.set("LegalCopyright", "Copyright (c) 2002 - 2026 CodeGator. All rights reserved.");
    res.set("OriginalFilename", "wiggler.exe");
    if let Err(err) = res.compile() {
        println!("cargo:warning=could not embed Windows resources: {err}");
    }
}
