fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=installer-hooks.nsh");
    tauri_build::build()
}