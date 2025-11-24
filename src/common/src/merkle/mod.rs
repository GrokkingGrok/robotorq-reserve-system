use sha2::{Sha256, Digest};

/// Compute SHA256 hash of concatenated left+right byte strings.
fn hash_pair(left: &str, right: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build a merkle root from an ordered slice of hex hashes.
/// Empty slice => empty string. Single hash => identity.
pub fn build_merkle_root(hashes: &[String]) -> String {
    if hashes.is_empty() {
        return String::new();
    }
    // Small: sequential; Large: parallel via rayon
    if hashes.len() <= 32 {
        return build_sequential(hashes);
    }
    build_parallel(hashes)
}

fn build_sequential(hashes: &[String]) -> String {
    let mut level: Vec<String> = hashes.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        for chunk in level.chunks(2) {
            let h = if chunk.len() == 2 { hash_pair(&chunk[0], &chunk[1]) } else { hash_pair(&chunk[0], &chunk[0]) };
            next.push(h);
        }
        level = next;
    }
    level[0].clone()
}

fn build_parallel(hashes: &[String]) -> String {
    use rayon::prelude::*;
    let mut level: Vec<String> = hashes.to_vec();
    while level.len() > 1 {
        level = level
            .par_chunks(2)
            .map(|chunk| if chunk.len() == 2 { hash_pair(&chunk[0], &chunk[1]) } else { hash_pair(&chunk[0], &chunk[0]) })
            .collect();
    }
    level[0].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_root() { assert_eq!(build_merkle_root(&[]), ""); }
    #[test]
    fn single_root() { assert_eq!(build_merkle_root(&["abc".into()]), "abc".to_string()); }
    #[test]
    fn two_hashes() {
        let a = "a".to_string();
        let b = "b".to_string();
        let expected = super::hash_pair(&a, &b);
        assert_eq!(build_merkle_root(&[a, b]), expected);
    }
    #[test]
    fn four_hashes() {
        let h = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        let ab = super::hash_pair("a", "b");
        let cd = super::hash_pair("c", "d");
        let expected = super::hash_pair(&ab, &cd);
        assert_eq!(build_merkle_root(&h), expected);
    }
    #[test]
    fn odd_hashes() {
        let h = vec!["a".into(), "b".into(), "c".into()];
        let ab = super::hash_pair("a", "b");
        let cc = super::hash_pair("c", "c");
        let expected = super::hash_pair(&ab, &cc);
        assert_eq!(build_merkle_root(&h), expected);
    }
    #[test]
    fn hash_pair_diff() { assert_ne!(super::hash_pair("a","b"), super::hash_pair("a","c")); }
}
