fn main() {
    println!("cargo:rerun-if-changed=razer-gui.rc");
    println!("cargo:rerun-if-changed=rhelper.ico");
    if cfg!(target_os = "windows") {
        if let Err(e) = embed_resource::compile("razer-gui.rc", embed_resource::NONE).manifest_optional() {
            println!("cargo:warning=embed-resource failed: {e}");
        }
    }
}
