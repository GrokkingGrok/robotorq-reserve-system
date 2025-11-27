use serde::{Deserialize, Serialize};
use crate::util::hashing::hash_bytes;

/// Abstraction for items that can be used as Merkle leaves.
pub trait MerkleHashable {
	/// Bytes to hash for the leaf. Should be a stable, canonical encoding.
	fn merkle_bytes(&self) -> Vec<u8>;
}

/// Hashing strategy for Merkle trees.
pub trait MerkleHash {
	/// Hash raw leaf bytes.
	fn hash_leaf(&self, leaf_bytes: &[u8]) -> [u8; 32];
	/// Hash an internal node from two child hashes.
	fn hash_node(&self, left: [u8; 32], right: [u8; 32]) -> [u8; 32];
}

/// Default Blake3-based hasher used across the system.
#[derive(Debug, Clone, Copy, Default)]
pub struct Blake3MerkleHash;

impl MerkleHash for Blake3MerkleHash {
	fn hash_leaf(&self, leaf_bytes: &[u8]) -> [u8; 32] {
		hash_bytes(leaf_bytes)
	}
	fn hash_node(&self, left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
		let mut buf = Vec::with_capacity(64);
		buf.extend_from_slice(&left);
		buf.extend_from_slice(&right);
		hash_bytes(&buf)
	}
}

/// Position of the sibling at each level for a proof path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SiblingPos {
	Left,
	Right,
}

/// Merkle proof consisting of sibling hashes and their positions up to the root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
	pub path: Vec<([u8; 32], SiblingPos)>,
	pub root: [u8; 32],
	pub leaf: [u8; 32],
	pub index: usize,
}

impl MerkleProof {
	/// Verify this proof against the root using the provided hasher.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
	pub leaves: Vec<[u8; 32]>,
	pub levels: Vec<Vec<[u8; 32]>>, // levels[0] = leaves, levels.last() contains root vector (len=1)
}

impl MerkleTree {
	pub fn root(&self) -> Option<[u8; 32]> {
		self.levels.last().and_then(|lvl| lvl.first().copied())
	}

	/// Build a proof for the leaf at `index`.
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
#[derive(Debug)]
pub struct MerkleBuilder<H: MerkleHash + Send + 'static> {
	hasher: H,
	leaves: Vec<[u8; 32]>,
}

impl<H: MerkleHash + Send + 'static> MerkleBuilder<H> {
	pub fn new(hasher: H) -> Self { Self { hasher, leaves: Vec::new() } }

	pub fn add_leaf_bytes(&mut self, bytes: &[u8]) {
		let h = self.hasher.hash_leaf(bytes);
		self.leaves.push(h);
	}

	pub fn add_leaf<T: MerkleHashable>(&mut self, item: &T) {
		self.add_leaf_bytes(&item.merkle_bytes());
	}

	/// Finalize synchronously.
	pub fn finalize_sync(self) -> MerkleTree {
		build_levels(self.leaves, self.hasher)
	}

	/// Finalize asynchronously. With feature `async-merkle`, runs in a blocking task on Tokio.
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
	pub fn build_from_slice_sync<T: MerkleHashable>(hasher: H, items: &[T]) -> MerkleTree where H: Clone {
		let mut b = MerkleBuilder::new(hasher);
		for it in items { b.add_leaf(it); }
		b.finalize_sync()
	}

	/// Convenience: build directly from a slice of items (async signature).
	pub async fn build_from_slice_async<T: MerkleHashable>(hasher: H, items: &[T]) -> MerkleTree where H: Clone {
		let mut b = MerkleBuilder::new(hasher);
		for it in items { b.add_leaf(it); }
		b.finalize_async().await
	}
}

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

	#[derive(Clone)]
	struct Demo(u32);
	impl MerkleHashable for Demo {
		fn merkle_bytes(&self) -> Vec<u8> { self.0.to_le_bytes().to_vec() }
	}

	#[test]
	fn merkle_build_and_proof_verify() {
		let hasher = Blake3MerkleHash::default();
		let items = vec![Demo(1), Demo(2), Demo(3), Demo(4), Demo(5)];
		let tree = MerkleBuilder::build_from_slice_sync(hasher, &items);
		let root = tree.root().unwrap();
		let proof = tree.proof_for(3).unwrap(); // item 4
		assert!(proof.verify(hasher));
		assert_eq!(proof.root, root);
	}
}
