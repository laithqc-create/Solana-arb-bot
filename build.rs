fn main() {
    // Skip Tauri's icon parsing - we'll use a default icon
    // This avoids the ico parsing error during build
    println!("cargo:rustc-env=TAURI_ENV_TARGET_TRIPLE=x86_64-pc-windows-msvc");
}
