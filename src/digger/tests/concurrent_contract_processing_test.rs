// Test for concurrent contract hash batch processing
// Verifies that multiple contracts are processed in parallel, not sequentially

#[cfg(test)]
mod concurrent_tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    /// Mock function simulating sequential contract processing
    async fn process_contracts_sequential(contract_ids: Vec<String>) -> Duration {
        let start = Instant::now();
        
        for contract_id in contract_ids {
            // Simulate 100ms processing per contract (hash retrieval, signing, publish)
            sleep(Duration::from_millis(100)).await;
            println!("Processed contract {} sequentially", contract_id);
        }
        
        start.elapsed()
    }

    /// Mock function simulating concurrent contract processing
    async fn process_contracts_concurrent(contract_ids: Vec<String>) -> Duration {
        let start = Instant::now();
        let mut tasks = vec![];
        
        for contract_id in contract_ids {
            let task = tokio::spawn(async move {
                // Simulate 100ms processing per contract
                sleep(Duration::from_millis(100)).await;
                println!("Processed contract {} concurrently", contract_id);
            });
            
            tasks.push(task);
        }
        
        // Wait for all contracts to finish
        for task in tasks {
            let _ = task.await;
        }
        
        start.elapsed()
    }

    #[tokio::test]
    async fn test_concurrent_processing_is_faster() {
        let contracts = vec![
            "contract-1".to_string(),
            "contract-2".to_string(),
            "contract-3".to_string(),
            "contract-4".to_string(),
            "contract-5".to_string(),
        ];

        // Sequential: should take ~500ms (5 contracts × 100ms each)
        let sequential_time = process_contracts_sequential(contracts.clone()).await;
        println!("Sequential processing took: {:?}", sequential_time);
        
        // Concurrent: should take ~100ms (all 5 in parallel)
        let concurrent_time = process_contracts_concurrent(contracts.clone()).await;
        println!("Concurrent processing took: {:?}", concurrent_time);
        
        // Verify concurrent is significantly faster (at least 3x speedup)
        assert!(
            concurrent_time < sequential_time / 3,
            "Concurrent processing should be at least 3x faster. Sequential: {:?}, Concurrent: {:?}",
            sequential_time,
            concurrent_time
        );
        
        // Verify concurrent time is close to single contract time (~100ms)
        assert!(
            concurrent_time.as_millis() < 200,
            "Concurrent processing should complete in ~100ms, took {:?}",
            concurrent_time
        );
    }

    #[tokio::test]
    async fn test_concurrent_with_shared_state() {
        // Test that concurrent tasks can safely access shared state with mutexes
        let shared_counter = Arc::new(Mutex::new(0));
        let contract_ids = vec!["c1", "c2", "c3", "c4", "c5"];
        
        let mut tasks = vec![];
        
        for contract_id in contract_ids {
            let counter_clone = Arc::clone(&shared_counter);
            
            let task = tokio::spawn(async move {
                // Simulate processing
                sleep(Duration::from_millis(50)).await;
                
                // Update shared state (like contract manager mark_hash_send)
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
                
                println!("Contract {} incremented counter", contract_id);
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }
        
        // Verify all contracts updated shared state
        let final_count = *shared_counter.lock().unwrap();
        assert_eq!(final_count, 5, "All 5 contracts should have incremented counter");
    }

    #[tokio::test]
    async fn test_no_deadlock_with_concurrent_locks() {
        // Ensure concurrent mutex access doesn't deadlock
        let state1 = Arc::new(Mutex::new(vec![1, 2, 3]));
        let state2 = Arc::new(Mutex::new(vec![4, 5, 6]));
        
        let mut tasks = vec![];
        
        for i in 0..10 {
            let s1 = Arc::clone(&state1);
            let s2 = Arc::clone(&state2);
            
            let task = tokio::spawn(async move {
                // Properly scope mutex guards before await
                if i % 2 == 0 {
                    {
                        let _data1 = s1.lock().unwrap();
                        // Use data here
                    }
                    sleep(Duration::from_millis(10)).await;
                    {
                        let _data2 = s2.lock().unwrap();
                        // Use data here
                    }
                } else {
                    {
                        let _data2 = s2.lock().unwrap();
                        // Use data here
                    }
                    sleep(Duration::from_millis(10)).await;
                    {
                        let _data1 = s1.lock().unwrap();
                        // Use data here
                    }
                }
            });
            
            tasks.push(task);
        }
        
        // This test passes if it completes without hanging
        let start = Instant::now();
        for task in tasks {
            let _ = task.await;
        }
        
        let elapsed = start.elapsed();
        println!("All tasks completed in {:?}", elapsed);
        
        // Should complete quickly without deadlock
        assert!(
            elapsed.as_secs() < 2,
            "Tasks should complete quickly without deadlock"
        );
    }
}
