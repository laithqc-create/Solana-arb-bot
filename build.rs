use tauri_build::build;

fn main() {
    // Remove icon from build context to avoid parsing errors
    // Tauri will use its default icon
    std::env::set_var("TAURI_SKIP_WINRES_ICON", "1");
    
    build();
}
