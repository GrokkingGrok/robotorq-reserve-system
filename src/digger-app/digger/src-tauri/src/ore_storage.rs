use crate::types::JouleTorqOre;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

#[derive(Clone)]
pub struct OreStorage {
    pub ores: Vec<JouleTorqOre>,
}

impl OreStorage {
    pub fn global() -> Arc<Mutex<Self>> {
        lazy_static! {
            static ref INSTANCE: Arc<Mutex<OreStorage>> =
                Arc::new(Mutex::new(OreStorage { ores: Vec::new() }));
        }
        INSTANCE.clone()
    }

    pub fn add_ore(&mut self, ore: JouleTorqOre) {
        self.ores.push(ore);
    }

    pub fn get_all(&self) -> Vec<JouleTorqOre> {
        self.ores.clone()
    }
}
