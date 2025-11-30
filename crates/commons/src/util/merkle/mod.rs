use serde::{Deserialize, Serialize};
use crate::util::hashing::hash_bytes;

/// Abstraction for items that can be used as Merkle leaves.
///
/// Types implementing this trait can be included in Merkle trees for cryptographic verification.
///
/// # Example
/// ```
/// use commons::util::merkle::MerkleHashable;
///
/// struct MyData(u32);
///
/// impl MerkleHashable for MyData {
///     fn merkle_bytes(&self) -> Vec<u8> {
///         self.0.to_le_bytes().to_vec()
///     }
/// }
/// ```
pub trait MerkleHashable {
	/// Bytes to hash for the leaf. Should be a stable, canonical encoding.
	fn merkle_bytes(&self) -> Vec<u8>;
}

/// Hashing strategy for Merkle trees.
///
/// This trait defines the interface for hashing operations used in Merkle tree construction,
/// allowing different cryptographic hash functions to be used.
///
/// # Example
/// ```
/// use commons::util::merkle::MerkleHash;
///
/// struct MyHasher;
///
/// impl MerkleHash for MyHasher {
///     fn hash_leaf(&self, leaf_bytes: &[u8]) -> [u8; 32] {
///         // Custom leaf hashing logic
///         [0u8; 32] // placeholder
///     }
///
///     fn hash_node(&self, left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
///         // Custom node hashing logic
///         [0u8; 32] // placeholder
///     }
/// }
/// ```
pub trait MerkleHash {
	/// Hash raw leaf bytes.
	fn hash_leaf(&self, leaf_bytes: &[u8]) -> [u8; 32];
	/// Hash an internal node from two child hashes.
	fn hash_node(&self, left: [u8; 32], right: [u8; 32]) -> [u8; 32];
}

/// Default Blake3-based hasher used across the system.
///
/// This is the standard hasher implementation using Blake3 for all Merkle tree operations.
///
/// # Example
/// ```
/// use commons::util::merkle::MerkleHashBuilder;
///
/// let hasher = MerkleHashBuilder::default();
/// let leaf_hash = hasher.hash_leaf(b"data");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct MerkleHashBuilder;

impl MerkleHash for MerkleHashBuilder {
	/// Hash raw leaf bytes using Blake3.
	fn hash_leaf(&self, leaf_bytes: &[u8]) -> [u8; 32] {
		hash_bytes(leaf_bytes)
	}
	/// Hash an internal node from two child hashes using Blake3.
	fn hash_node(&self, left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
		let mut buf = Vec::with_capacity(64);
		buf.extend_from_slice(&left);
		buf.extend_from_slice(&right);
		hash_bytes(&buf)
	}
}

/// Position of the sibling at each level for a proof path.
///
/// Used in Merkle proofs to indicate whether the sibling hash comes from the left or right child.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SiblingPos {
	Left,
	Right,
}

/// Merkle proof consisting of sibling hashes and their positions up to the root.
///
/// A Merkle proof allows verification that a particular leaf is part of a Merkle tree
/// without needing the entire tree.
///
/// # Fields
///
/// * `path` - Vector of (sibling_hash, position) pairs for each level
/// * `root` - The root hash of the Merkle tree
/// * `leaf` - The hash of the leaf being proven
/// * `index` - The index of the leaf in the original tree
///
/// # Example
/// ```
/// use commons::util::merkle::{MerkleProof, MerkleHashBuilder};
///
/// // Proof would typically be created by MerkleTree::proof_for()
/// let proof = MerkleProof {
///     path: vec![],
///     root: [0u8; 32],
///     leaf: [0u8; 32],
///     index: 0,
/// };
///
/// let hasher = MerkleHashBuilder::default();
/// let is_valid = proof.verify(hasher);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
	pub path: Vec<([u8; 32], SiblingPos)>,
	pub root: [u8; 32],
	pub leaf: [u8; 32],
	pub index: usize,
}

impl MerkleProof {
	/// Verify this proof against the root using the provided hasher.
	///
	/// # Fields
	///
	/// * `hasher` - The hashing strategy to use for verification
	///
	/// # Returns
	///
	/// `true` if the proof is valid and the leaf is correctly included in the tree, `false` otherwise.
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleProof, MerkleHashBuilder};
	///
	/// let proof = MerkleProof {
	///     path: vec![],
	///     root: [0u8; 32],
	///     leaf: [0u8; 32],
	///     index: 0,
	/// };
	///
	/// let hasher = MerkleHashBuilder::default();
	/// assert!(proof.verify(hasher)); // Would be true for a valid proof
	/// ```
	pub fn verify<H: MerkleHash>(&self, hasher: H) -> bool {
		let mut acc = self.leaf;
		for (sib, pos) in &self.path {
			acc = match pos {
				SiblingPos::Left => hasher.hash_node(*sib, acc),
				SiblingPos::Right => hasher.hash_node(acc, *sib),
			};
		}
		acc == self.root
	}
}

/// A built Merkle tree capturing all levels for proof generation.
///
/// Contains the complete tree structure including all intermediate hashes,
/// allowing for efficient proof generation.
///
/// # Fields
///
/// * `leaves` - The leaf hashes of the tree
/// * `levels` - All levels of the tree, where levels[0] are leaves and levels.last() contains the root
///
/// # Example
/// ```
/// use commons::util::merkle::{MerkleTree, MerkleBuilder, MerkleHashBuilder};
///
/// let hasher = MerkleHashBuilder::default();
/// let mut builder = MerkleBuilder::new(hasher);
/// builder.add_leaf_bytes(b"data1");
/// builder.add_leaf_bytes(b"data2");
/// let tree = builder.finalize_sync();
///
/// let root = tree.root().unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
	pub leaves: Vec<[u8; 32]>,
	pub levels: Vec<Vec<[u8; 32]>>, // levels[0] = leaves, levels.last() contains root vector (len=1)
}

impl MerkleTree {
	/// Get the root hash of this Merkle tree.
	///
	/// # Returns
	///
	/// `Some(root_hash)` if the tree has been built, `None` if the tree is empty.
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleTree, MerkleBuilder, MerkleHashBuilder};
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let mut builder = MerkleBuilder::new(hasher);
	/// builder.add_leaf_bytes(b"data");
	/// let tree = builder.finalize_sync();
	///
	/// let root = tree.root().unwrap();
	/// ```
	pub fn root(&self) -> Option<[u8; 32]> {
		self.levels.last().and_then(|lvl| lvl.first().copied())
	}

	/// Build a proof for the leaf at `index`.
	///
	/// # Fields
	///
	/// * `index` - The index of the leaf to generate a proof for
	///
	/// # Returns
	///
	/// `Some(MerkleProof)` if the index is valid, `None` if the index is out of bounds or tree is empty.
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleTree, MerkleBuilder, MerkleHashBuilder};
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let mut builder = MerkleBuilder::new(hasher);
	/// builder.add_leaf_bytes(b"data1");
	/// builder.add_leaf_bytes(b"data2");
	/// let tree = builder.finalize_sync();
	///
	/// let proof = tree.proof_for(0).unwrap();
	/// assert!(proof.verify(hasher));
	/// ```
	pub fn proof_for(&self, index: usize) -> Option<MerkleProof> {
		if self.leaves.is_empty() || index >= self.leaves.len() { return None; }
		let mut idx = index;
		let mut path: Vec<([u8; 32], SiblingPos)> = Vec::new();
		for level in &self.levels {
			if level.len() == 1 { break; }
			let is_right = idx % 2 == 1;
			let sib_idx = if is_right { idx - 1 } else { idx + 1 };
			let sibling = if sib_idx < level.len() { level[sib_idx] } else { level[idx] }; // duplicate last if odd
			path.push((sibling, if is_right { SiblingPos::Left } else { SiblingPos::Right }));
			idx /= 2;
		}
		Some(MerkleProof { path, root: self.root()?, leaf: self.leaves[index], index })
	}
}

/// Streaming-friendly Merkle builder that can be shared across refinery, mint, and vault.
///
/// Allows incremental construction of Merkle trees by adding leaves one at a time,
/// with both synchronous and asynchronous finalization options.
///
/// # Fields
///
/// * `hasher` - The hashing strategy to use for tree construction
/// * `leaves` - Accumulated leaf hashes
///
/// # Example
/// ```
/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder};
///
/// let hasher = MerkleHashBuilder::default();
/// let mut builder = MerkleBuilder::new(hasher);
/// builder.add_leaf_bytes(b"data1");
/// builder.add_leaf_bytes(b"data2");
/// let tree = builder.finalize_sync();
/// ```
#[derive(Debug)]
pub struct MerkleBuilder<H: MerkleHash + Send + 'static> {
	hasher: H,
	leaves: Vec<[u8; 32]>,
}

impl<H: MerkleHash + Send + 'static> MerkleBuilder<H> {
	/// Create a new Merkle builder with the given hasher.
	///
	/// # Fields
	///
	/// * `hasher` - The hashing strategy to use for tree construction
	///
	/// # Returns
	///
	/// A new `MerkleBuilder` instance ready to accept leaves.
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder};
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let builder = MerkleBuilder::new(hasher);
	/// ```
	pub fn new(hasher: H) -> Self { Self { hasher, leaves: Vec::new() } }

	/// Add a leaf to the tree by providing its raw bytes.
	///
	/// # Fields
	///
	/// * `bytes` - The raw bytes to hash as a leaf
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder};
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let mut builder = MerkleBuilder::new(hasher);
	/// builder.add_leaf_bytes(b"my data");
	/// ```
	pub fn add_leaf_bytes(&mut self, bytes: &[u8]) {
		let h = self.hasher.hash_leaf(bytes);
		self.leaves.push(h);
	}

	/// Add a leaf to the tree from a `MerkleHashable` item.
	///
	/// # Fields
	///
	/// * `item` - The item to add as a leaf, must implement `MerkleHashable`
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder, MerkleHashable};
	///
	/// struct MyItem(u32);
	///
	/// impl MerkleHashable for MyItem {
	///     fn merkle_bytes(&self) -> Vec<u8> {
	///         self.0.to_le_bytes().to_vec()
	///     }
	/// }
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let mut builder = MerkleBuilder::new(hasher);
	/// let item = MyItem(42);
	/// builder.add_leaf(&item);
	/// ```
	pub fn add_leaf<T: MerkleHashable>(&mut self, item: &T) {
		self.add_leaf_bytes(&item.merkle_bytes());
	}

	/// Finalize synchronously.
	///
	/// # Returns
	///
	/// The completed `MerkleTree` with all levels computed.
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder};
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let mut builder = MerkleBuilder::new(hasher);
	/// builder.add_leaf_bytes(b"data");
	/// let tree = builder.finalize_sync();
	/// ```
	pub fn finalize_sync(self) -> MerkleTree {
		build_levels(self.leaves, self.hasher)
	}

	/// Finalize asynchronously. With feature `async-merkle`, runs in a blocking task on Tokio.
	///
	/// # Returns
	///
	/// The completed `MerkleTree` with all levels computed.
	///
	/// # Panics
	///
	/// Panics if the internal blocking task fails (e.g., due to Tokio runtime issues).
	///
	/// # Example
	/// ```no_run
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder};
	///
	/// # async fn example() {
	/// let hasher = MerkleHashBuilder::default();
	/// let mut builder = MerkleBuilder::new(hasher);
	/// builder.add_leaf_bytes(b"data");
	/// let tree = builder.finalize_async().await;
	/// # }
	/// ```
	pub async fn finalize_async(self) -> MerkleTree {
		#[cfg(feature = "async-merkle")]
		{
			let leaves = self.leaves;
			let hasher = self.hasher;
			tokio::task::spawn_blocking(move || build_levels(leaves, hasher)).await.unwrap()
		}
		#[cfg(not(feature = "async-merkle"))]
		{
			// Fallback to synchronous computation if async feature is disabled.
			build_levels(self.leaves, self.hasher)
		}
	}

	/// Convenience: build directly from a slice of items (sync).
	///
	/// # Fields
	///
	/// * `hasher` - The hashing strategy to use
	/// * `items` - Slice of items to include in the tree
	///
	/// # Returns
	///
	/// The completed `MerkleTree` containing all items.
	///
	/// # Example
	/// ```
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder, MerkleHashable};
	///
	/// struct MyItem(u32);
	///
	/// impl MerkleHashable for MyItem {
	///     fn merkle_bytes(&self) -> Vec<u8> {
	///         self.0.to_le_bytes().to_vec()
	///     }
	/// }
	///
	/// let hasher = MerkleHashBuilder::default();
	/// let items = vec![MyItem(1), MyItem(2), MyItem(3)];
	/// let tree = MerkleBuilder::build_from_slice_sync(hasher, &items);
	/// ```
	pub fn build_from_slice_sync<T: MerkleHashable>(hasher: H, items: &[T]) -> MerkleTree where H: Clone {
		let mut b = MerkleBuilder::new(hasher);
		for it in items { b.add_leaf(it); }
		b.finalize_sync()
	}

	/// Convenience: build directly from a slice of items (async signature).
	///
	/// # Fields
	///
	/// * `hasher` - The hashing strategy to use
	/// * `items` - Slice of items to include in the tree
	///
	/// # Returns
	///
	/// The completed `MerkleTree` containing all items.
	///
	/// # Panics
	///
	/// Panics if the internal blocking task fails (e.g., due to Tokio runtime issues).
	///
	/// # Example
	/// ```no_run
	/// use commons::util::merkle::{MerkleBuilder, MerkleHashBuilder, MerkleHashable};
	///
	/// struct MyItem(u32);
	///
	/// impl MerkleHashable for MyItem {
	///     fn merkle_bytes(&self) -> Vec<u8> {
	///         self.0.to_le_bytes().to_vec()
	///     }
	/// }
	///
	/// # async fn example() {
	/// let hasher = MerkleHashBuilder::default();
	/// let items = vec![MyItem(1), MyItem(2), MyItem(3)];
	/// let tree = MerkleBuilder::build_from_slice_async(hasher, &items).await;
	/// # }
	/// ```
	pub async fn build_from_slice_async<T: MerkleHashable>(hasher: H, items: &[T]) -> MerkleTree where H: Clone {
		let mut b = MerkleBuilder::new(hasher);
		for it in items { b.add_leaf(it); }
		b.finalize_async().await
	}
}

/// Builds the internal levels of a Merkle tree from leaf hashes.
///
/// This function constructs the complete Merkle tree structure by iteratively
/// hashing pairs of nodes until a single root hash is reached.
///
/// # Arguments
/// * `leaves` - Vector of 32-byte leaf hashes
/// * `hasher` - Hash implementation to use for node hashing
///
/// # Returns
/// A complete `MerkleTree` with all levels computed
///
/// # Example
/// ```
/// use robotorq_commons::util::merkle::{build_levels, MerkleHashBuilder};
///
/// let leaves = vec![[0u8; 32], [1u8; 32]];
/// let hasher = MerkleHashBuilder::default();
/// let tree = build_levels(leaves, hasher);
/// assert_eq!(tree.levels.len(), 2); // leaves + root level
/// ```
fn build_levels<H: MerkleHash>(leaves: Vec<[u8; 32]>, hasher: H) -> MerkleTree {
	if leaves.is_empty() { return MerkleTree { leaves, levels: vec![vec![]] }; }
	let mut levels: Vec<Vec<[u8; 32]>> = Vec::new();
	levels.push(leaves.clone());
	loop {
		let cur = levels.last().unwrap();
		if cur.len() == 1 { break; }
		let mut next: Vec<[u8; 32]> = Vec::with_capacity((cur.len() + 1) / 2);
		let mut i = 0;
		while i < cur.len() {
			let left = cur[i];
			let right = if i + 1 < cur.len() { cur[i + 1] } else { cur[i] };
			next.push(hasher.hash_node(left, right));
			i += 2;
		}
		levels.push(next);
	}
	MerkleTree { leaves, levels }
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Test Merkle tree building and proof verification.
	///
	/// This test verifies that a Merkle tree can be built from multiple items
	/// and that proofs can be generated and verified correctly.
	/// Test Merkle tree building and proof verification.
	///
	/// This test verifies that a Merkle tree can be built from multiple items
	/// and that proofs can be generated and verified correctly.
	#[derive(Clone)]
	struct Demo(u32);
	impl MerkleHashable for Demo {
		fn merkle_bytes(&self) -> Vec<u8> { self.0.to_le_bytes().to_vec() }
	}

	#[test]
	fn merkle_build_and_proof_verify() {
		let hasher = MerkleHashBuilder::default();
		let items = vec![Demo(1), Demo(2), Demo(3), Demo(4), Demo(5)];
		let tree = MerkleBuilder::build_from_slice_sync(hasher, &items);
		let root = tree.root().unwrap();
		let proof = tree.proof_for(3).unwrap(); // item 4
		assert!(proof.verify(MerkleHashBuilder::default()));
		assert_eq!(proof.root, root);
	}
}
