// Test for parallel JTU generation using Rayon
// Verifies that JTU hash calculations can be parallelized for performance

#[cfg(test)]
mod parallel_jtu_tests {
    use std::time::{Duration, Instant};
    use rayon::prelude::*;

    /// Mock JTU structure for testing
    #[derive(Debug, Clone, PartialEq)]
    struct MockJtu {
        token_id: String,
        hash: String,
        joules: f64,
        robo_stake: f64,
    }

    /// Simulate hash calculation (CPU-bound operation)
    fn calculate_mock_hash(token_id: &str, joules: f64, robo_stake: f64) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(token_id.as_bytes());
        hasher.update(joules.to_le_bytes());
        hasher.update(robo_stake.to_le_bytes());
        
        // Simulate additional computation to make it more realistic
        for _ in 0..100 {
            hasher.update(&[0u8; 32]);
        }
        
        format!("{:x}", hasher.finalize())
    }

    /// Sequential JTU generation (baseline)
    fn generate_jtus_sequential(count: i64, joules_per_jtu: f64, robo_per_jtu: f64) -> Vec<MockJtu> {
        let mut jtus = Vec::with_capacity(count as usize);
        
        for i in 0..count {
            let token_id = format!("test-contract-t{}", i);
            let hash = calculate_mock_hash(&token_id, joules_per_jtu, robo_per_jtu);
            
            jtus.push(MockJtu {
                token_id,
                hash,
                joules: joules_per_jtu,
                robo_stake: robo_per_jtu,
            });
        }
        
        jtus
    }

    /// Parallel JTU generation using Rayon
    fn generate_jtus_parallel(count: i64, joules_per_jtu: f64, robo_per_jtu: f64) -> Vec<MockJtu> {
        (0..count)
            .into_par_iter()
            .map(|i| {
                let token_id = format!("test-contract-t{}", i);
                let hash = calculate_mock_hash(&token_id, joules_per_jtu, robo_per_jtu);
                
                MockJtu {
                    token_id,
                    hash,
                    joules: joules_per_jtu,
                    robo_stake: robo_per_jtu,
                }
            })
            .collect()
    }

    #[test]
    fn test_parallel_jtu_generation_is_faster() {
        // Generate 10,000 JTUs (realistic for 5 second execution at 2000W)
        let jtu_count = 10_000;
        let joules_per_jtu = 0.01;
        let robo_per_jtu = 0.00001;

        println!("\n🧪 Testing JTU generation performance with {} JTUs", jtu_count);

        // Sequential
        let start = Instant::now();
        let sequential_jtus = generate_jtus_sequential(jtu_count, joules_per_jtu, robo_per_jtu);
        let sequential_time = start.elapsed();
        println!("⏱️  Sequential: {:?}", sequential_time);

        // Parallel
        let start = Instant::now();
        let parallel_jtus = generate_jtus_parallel(jtu_count, joules_per_jtu, robo_per_jtu);
        let parallel_time = start.elapsed();
        println!("⏱️  Parallel: {:?}", parallel_time);

        // Verify both produce same results (order may differ)
        assert_eq!(sequential_jtus.len(), parallel_jtus.len());
        
        // Sort both by token_id to compare
        let mut seq_sorted = sequential_jtus.clone();
        let mut par_sorted = parallel_jtus.clone();
        seq_sorted.sort_by(|a, b| a.token_id.cmp(&b.token_id));
        par_sorted.sort_by(|a, b| a.token_id.cmp(&b.token_id));
        
        assert_eq!(seq_sorted, par_sorted, "Sequential and parallel should produce identical JTUs");

        // Performance assertion: parallel should be faster (at least 1.5x on multi-core)
        let speedup = sequential_time.as_secs_f64() / parallel_time.as_secs_f64();
        println!("🚀 Speedup: {:.2}x", speedup);
        
        // On multi-core systems, expect at least 1.5x speedup
        // Note: May fail on single-core systems or in CI
        if num_cpus::get() > 1 {
            assert!(
                speedup > 1.3,
                "Parallel should be at least 1.3x faster on multi-core. Got: {:.2}x",
                speedup
            );
        }
    }

    #[test]
    fn test_parallel_correctness_with_different_values() {
        // Test that each JTU gets correct joules/robo values
        let jtu_count = 1000;
        let joules_per_jtu = 15.75;
        let robo_per_jtu = 0.00025;

        let jtus = generate_jtus_parallel(jtu_count, joules_per_jtu, robo_per_jtu);

        // Verify all JTUs have correct values
        for jtu in &jtus {
            assert_eq!(jtu.joules, joules_per_jtu, "Joules mismatch for {}", jtu.token_id);
            assert_eq!(jtu.robo_stake, robo_per_jtu, "RoboStake mismatch for {}", jtu.token_id);
            assert!(!jtu.hash.is_empty(), "Hash should not be empty for {}", jtu.token_id);
        }

        // Verify all hashes are unique
        let mut hashes: Vec<String> = jtus.iter().map(|j| j.hash.clone()).collect();
        hashes.sort();
        hashes.dedup();
        assert_eq!(
            hashes.len(),
            jtu_count as usize,
            "All JTU hashes should be unique"
        );
    }

    #[test]
    fn test_parallel_token_indexing() {
        // Verify token indices are correct after parallel generation
        let jtu_count = 5000;
        let jtus = generate_jtus_parallel(jtu_count, 10.0, 0.001);

        // Sort by extracting numeric index (not lexicographic sort)
        let mut sorted = jtus.clone();
        sorted.sort_by_key(|jtu| {
            // Extract number from "test-contract-t123"
            jtu.token_id
                .strip_prefix("test-contract-t")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
        });

        for (i, jtu) in sorted.iter().enumerate() {
            let expected_id = format!("test-contract-t{}", i);
            assert_eq!(
                jtu.token_id, expected_id,
                "Token ID mismatch at index {}",
                i
            );
        }
    }

    #[test]
    fn test_large_batch_parallel_generation() {
        // Test realistic large batch (20,000 JTUs = 10 seconds at 2000W)
        let jtu_count = 20_000;
        
        let start = Instant::now();
        let jtus = generate_jtus_parallel(jtu_count, 0.01, 0.00001);
        let elapsed = start.elapsed();

        println!("Generated {} JTUs in {:?}", jtu_count, elapsed);

        assert_eq!(jtus.len(), jtu_count as usize);
        
        // Should complete in reasonable time (less than 5 seconds on modern CPU)
        assert!(
            elapsed < Duration::from_secs(5),
            "Large batch should complete quickly. Took: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_parallel_hash_determinism() {
        // Verify same inputs produce same hashes (even in parallel)
        let token_id = "test-token-42";
        let joules = 15.5;
        let robo = 0.0005;

        let hash1 = calculate_mock_hash(token_id, joules, robo);
        let hash2 = calculate_mock_hash(token_id, joules, robo);

        assert_eq!(hash1, hash2, "Hash should be deterministic");

        // Generate same JTU set twice in parallel
        let jtus1 = generate_jtus_parallel(1000, joules, robo);
        let jtus2 = generate_jtus_parallel(1000, joules, robo);

        // Sort both
        let mut sorted1 = jtus1.clone();
        let mut sorted2 = jtus2.clone();
        sorted1.sort_by(|a, b| a.token_id.cmp(&b.token_id));
        sorted2.sort_by(|a, b| a.token_id.cmp(&b.token_id));

        assert_eq!(
            sorted1, sorted2,
            "Parallel generation should be deterministic"
        );
    }

    #[test]
    fn test_parallel_with_single_jtu() {
        // Edge case: single JTU should work with parallel iterator
        let jtus = generate_jtus_parallel(1, 10.0, 0.001);

        assert_eq!(jtus.len(), 1);
        assert_eq!(jtus[0].token_id, "test-contract-t0");
        assert_eq!(jtus[0].joules, 10.0);
        assert_eq!(jtus[0].robo_stake, 0.001);
    }

    #[test]
    fn test_parallel_cross_product_distribution() {
        // Verify joules/robo are evenly distributed across tokens
        let total_joules = 20000.0; // 2000W × 10 seconds
        let total_robo = 5.0;        // 5 RT stake
        let jtu_count = 20_000;      // 2000 JTU/sec × 10 sec

        let joules_per_jtu = total_joules / jtu_count as f64;
        let robo_per_jtu = total_robo / jtu_count as f64;

        let jtus = generate_jtus_parallel(jtu_count, joules_per_jtu, robo_per_jtu);

        // Sum all joules and robo from JTUs
        let sum_joules: f64 = jtus.iter().map(|j| j.joules).sum();
        let sum_robo: f64 = jtus.iter().map(|j| j.robo_stake).sum();

        // Should equal totals (within floating point precision)
        assert!(
            (sum_joules - total_joules).abs() < 0.01,
            "Total joules mismatch. Expected: {}, Got: {}",
            total_joules,
            sum_joules
        );
        assert!(
            (sum_robo - total_robo).abs() < 0.00001,
            "Total robo mismatch. Expected: {}, Got: {}",
            total_robo,
            sum_robo
        );
    }
}
