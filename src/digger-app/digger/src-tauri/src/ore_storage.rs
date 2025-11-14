// This file is the **TREASURE VAULT** — it stores all the digital gold (Ore) the robots make

use crate::types::JouleTorqOre;  // Import the "report card" that robots create
use std::sync::{Arc, Mutex};     // Tools to safely share the vault
use lazy_static::lazy_static;    // Magic to create one global vault

// ────────────────────────────────────────────────────────────────
// THE TREASURE VAULT (OreStorage)
// ────────────────────────────────────────────────────────────────

/// This is like a **giant safe** that holds every piece of Ore the robots make
#[derive(Clone)]
pub struct OreStorage {
    /// A list (like a notebook) of every report card
    /// Each entry is one "Ore" from a robot
    pub ores: Vec<JouleTorqOre>,
}

impl OreStorage {
    /// Create **one global vault** that everyone shares
    /// (Like a bank — only one vault for the whole system!)
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<OreStorage>> =
                Arc::new(Mutex::new(OreStorage { ores: Vec::new() }));
        }
        INSTANCE.clone()
    }

    /// Add a new **Ore report** to the vault
    /// Like putting a gold bar in the safe
    pub fn add_ore(&mut self, ore: JouleTorqOre) {
        self.ores.push(ore);  // Add to the end of the list
    }

    /// Get a **copy of every Ore** in the vault
    /// Like making a photocopy of the entire notebook
    #[allow(dead_code)] // Future use: dashboard ore history display
    pub fn get_all(&self) -> Vec<JouleTorqOre> {
        self.ores.clone()  // Return a fresh copy
    }
}