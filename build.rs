fn main() {
    println!("cargo:rerun-if-changed=razer-gui.rc");
    println!("cargo:rerun-if-changed=rhelper.ico");

    if cfg!(target_os = "windows") {
        // embed-resource 3.x: mark manifest as optional and unwrap the result so failures show clearly
        if let Err(e) = embed_resource::compile("razer-gui.rc", embed_resource::NONE)
            .manifest_optional()
        {
            eprintln!("embed-resource failed: {e}");
        }
    }
}
