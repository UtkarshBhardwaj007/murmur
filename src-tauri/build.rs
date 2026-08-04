fn main() {
    // The statically linked onnxruntime inside sherpa-onnx uses ETW and
    // registry APIs on Windows; sherpa-rs-sys forgets to link advapi32.
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=advapi32");

    tauri_build::build()
}
