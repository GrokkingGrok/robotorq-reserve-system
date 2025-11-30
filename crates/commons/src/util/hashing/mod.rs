//! Cryptographic hashing utilities for the RoboTorq Reserve System.
//!
//! This module provides Blake3-based hashing functions used throughout the system
//! for cryptographic operations, data integrity verification, and Merkle tree construction.
//!
//! # Hashing Strategy
//!
//! All hashing in RoboTorq uses Blake3, a fast and secure cryptographic hash function.
//! The system uses 32-byte (256-bit) hash outputs for consistency with cryptographic standards.
//!
//! # Usage
//!
//! For structured data that implements `serde::Serialize`:
//! ```
//! # use serde::Serialize;
//! # #[derive(Serialize)]
//! # struct MyData { value: u32 }
//! # let data = MyData { value: 42 };
//! let hash = commons::util::hashing::hash_struct(&data);
//! ```
//!
//! For raw bytes:
//! ```
//! let data = b"some data";
//! let hash = commons::util::hashing::hash_bytes(data);
//! ```

use blake3::Hasher;
use serde::Serialize;

/// Hash a serializable struct using Blake3.
///
/// Converts the input value to JSON format and computes a 32-byte Blake3 hash.
/// This is the standard way to hash structured data in the RoboTorq system.
///
/// # Arguments
///
/// * `value` - Any type that implements `serde::Serialize`
///
/// # Returns
///
/// A 32-byte Blake3 hash of the JSON representation of the input value.
///
/// # Examples
///
/// ```
/// # use serde::Serialize;
/// # #[derive(Serialize)]
/// # struct Token { id: u32, amount: u64 }
/// # let token = Token { id: 1, amount: 100 };
/// let hash = commons::util::hashing::hash_struct(&token);
/// assert_eq!(hash.len(), 32);
/// ```
///
/// # Panics
///
/// This function does not panic under normal operation. The internal `expect` call
/// is for JSON serialization, which should be infallible for the types used in this system.
pub fn hash_struct<T: Serialize>(value: &T) -> [u8; 32] {
    let json = serde_json::to_vec(value).expect("serialization");
    let mut hasher = Hasher::new();
    hasher.update(&json);
    let output = hasher.finalize();
    *output.as_bytes()
}

/// Hash raw bytes directly using Blake3.
///
/// Computes a 32-byte Blake3 hash of the input byte slice without any preprocessing.
/// Use this for hashing binary data, pre-computed values, or when you want direct
/// byte-level hashing without JSON serialization overhead.
///
/// # Arguments
///
/// * `bytes` - The byte slice to hash
///
/// # Returns
///
/// A 32-byte Blake3 hash of the input bytes.
///
/// # Examples
///
/// ```
/// let data = b"Hello, world!";
/// let hash = commons::util::hashing::hash_bytes(data);
/// assert_eq!(hash.len(), 32);
///
/// // Hashing binary data
/// let binary_data = &[0x01, 0x02, 0x03, 0x04];
/// let binary_hash = commons::util::hashing::hash_bytes(binary_data);
/// ```
///
/// # Panics
///
/// This function does not panic. It operates directly on byte slices and has no
/// failure modes under normal operation.
pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    let output = hasher.finalize();
    *output.as_bytes()
}
