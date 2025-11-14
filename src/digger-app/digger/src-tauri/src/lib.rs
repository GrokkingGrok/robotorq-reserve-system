// ────────────────────────────────────────────────────────────────
// THIS IS THE **BRAIN** OF THE APP — Where Rust and JavaScript meet
// ────────────────────────────────────────────────────────────────

// Crypto module for post-quantum signatures (TODO #2)
pub mod crypto;

// Refinery HTTP client (TODO #4)
pub mod refinery_client;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

/// This is a **magic button** you can press from the screen
/// When JavaScript says "greet('Jon')", this runs and sends a message back
#[tauri::command]
fn greet(name: &str) -> String {
    // Make a friendly message using the name
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// This is the **"Start the Engine"** function
/// It launches the whole app when you open it
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()                    // Start building the app
        .plugin(tauri_plugin_opener::init())     // Add a helper to open links
        .invoke_handler(tauri::generate_handler![greet])  // Connect the "greet" button
        .run(tauri::generate_context!())         // Launch the window
        .expect("error while running tauri application"); // Stop if something breaks
}