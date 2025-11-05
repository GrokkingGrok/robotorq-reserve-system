// -------------------------------
// Import the wasm_bindgen prelude
// -------------------------------
use wasm_bindgen::prelude::*;
// Provides macros and types for communicating between Rust and JavaScript in WebAssembly.

// -------------------------------
// Declare external JavaScript functions
// -------------------------------
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    // Bind to JavaScript's console.log function for logging from Rust.

    #[wasm_bindgen(js_namespace = window)]
    fn alert(s: &str);
    // Bind to JavaScript's window.alert function to show pop-ups.
}

// -------------------------------
// Export Rust functions callable from JavaScript
// -------------------------------
#[wasm_bindgen]
pub fn double_value(x: i32) -> i32 {
    // A simple Rust function that doubles a number.
    // This function can be called directly from JS after WASM is loaded.
    x * 2
}

#[wasm_bindgen]
pub fn greet_user(name: &str) {
    // Another Rust function callable from JS.
    // Combines Rust logic with JS alert to greet a user dynamically.
    let message = format!("Hello, {}! Welcome to RoboTorq Wallet.", name);
    alert(&message);
}

// -------------------------------
// Entry point for the WASM module
// -------------------------------
#[wasm_bindgen(start)]
pub fn main() {
    // Automatically called when the WASM module is loaded.

    log("RoboTorq Wallet WASM loaded!");
    // Simple console log to confirm initialization.

    let initial_balance = 100;
    log(&format!("Initial balance: {}", initial_balance));
    // Demonstrates using Rust logic to prepare data and log it.

    let doubled = double_value(initial_balance);
    log(&format!("Doubled balance: {}", doubled));
    // Shows calling an internal Rust function and logging the result.

    // Optional: alert the user that the wallet is ready
    alert("Wallet initialized successfully!");
}

// -------------------------------
// Notes / best practices
// -------------------------------
// 1. Keep console logging minimal in production for performance.
// 2. Use #[wasm_bindgen] on Rust functions you need to call from JavaScript.
// 3. Use extern blocks to bind to JS APIs like console.log, alert, or custom JS functions.
// 4. Format strings in Rust using `format!` for safe and readable concatenation.
// 5. Initialize all app state in the #[wasm_bindgen(start)] function to avoid race conditions.
