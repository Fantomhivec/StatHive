fn main() {
    println!("cargo:rerun-if-changed=../VERSION");
    tauri_build::build()
}
