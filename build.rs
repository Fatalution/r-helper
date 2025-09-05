fn main() {
    println!("cargo:rerun-if-changed=razer-gui.rc");
    println!("cargo:rerun-if-changed=rhelper.ico");
    
    // Compile Windows resources (icon)
    if cfg!(target_os = "windows") {
        embed_resource::compile("razer-gui.rc", embed_resource::NONE);
    }
}
