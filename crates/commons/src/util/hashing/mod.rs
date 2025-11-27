use blake3::Hasher;
use serde::Serialize;

pub fn hash_struct<T: Serialize>(value: &T) -> [u8; 32] {
    let json = serde_json::to_vec(value).expect("serialization");
    let mut hasher = Hasher::new();
    hasher.update(&json);
    let output = hasher.finalize();
    *output.as_bytes()
}

pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    let output = hasher.finalize();
    *output.as_bytes()
}
