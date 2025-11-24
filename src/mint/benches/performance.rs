use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::SystemTime;

use common::merkle; // migrated shared merkle implementation
use mint::engine::ingot_processor::IngotProcessor;
use mint::engine::batcher::Batcher;
use mint::metrics::MintMetrics;
use mint::models::token_torq_ingot::TokenTorqIngot;
use mint::models::robotorq_certificate::RoboTorqCertificate;

fn create_test_ingot(id: &str, contract: &str) -> TokenTorqIngot {
    TokenTorqIngot {
        ingot_id: id.to_string(),
        contract_id: contract.to_string(),
        joules_total: 1000.0,
        robostake_total_micro_rt: 50000,
        unit_count: 3600,
        merkle_root: "test_root".to_string(),
        unit_hashes: vec!["hash1".to_string(), "hash2".to_string()],
        timestamp: SystemTime::now(),
        signature: vec![1, 2, 3],
    }
}

fn create_test_certificate(id: &str, contract: &str) -> RoboTorqCertificate {
    RoboTorqCertificate {
        cert_id: id.to_string(),
        robotorq_proof_id: format!("proof-{}", id),
        merkle_root: "test_root".to_string(),
        contract_ids: vec![contract.to_string()],
        timestamp_nanos: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64,
        hash: "test_hash".to_string(),
        status: mint::models::robotorq_certificate::CertStatus::Digital,
        bearer_bond_id: None,
        total_jouletorq: 3600 * 1000,
        total_stake_jouletorq: 3600 * 500,
        total_triple: common::triples::jouletorq_to_triple(3600 * 1000),
    }
}

fn bench_merkle_root_single(c: &mut Criterion) {
    let hashes = vec!["abc123".to_string()];

    c.bench_function("merkle_root_single", |b| {
        b.iter(|| {
            let result = merkle::build_merkle_root(black_box(&hashes));
            black_box(result);
        });
    });
}

fn bench_merkle_root_1000(c: &mut Criterion) {
    let hashes: Vec<String> = (0..1000).map(|i| format!("hash_{}", i)).collect();

    c.bench_function("merkle_root_1000", |b| {
        b.iter(|| {
            let result = merkle::build_merkle_root(black_box(&hashes));
            black_box(result);
        });
    });
}

fn bench_merkle_root_10000(c: &mut Criterion) {
    let hashes: Vec<String> = (0..10000).map(|i| format!("hash_{}", i)).collect();

    c.bench_function("merkle_root_10000", |b| {
        b.iter(|| {
            let result = merkle::build_merkle_root(black_box(&hashes));
            black_box(result);
        });
    });
}

fn bench_ingot_processor_single(c: &mut Criterion) {
    let (tx, _rx) = mpsc::channel(10);
    let metrics = Arc::new(MintMetrics::new());
    let processor = IngotProcessor::new(tx, Arc::clone(&metrics));
    let ingot = create_test_ingot("test-1", "contract-1");

    c.bench_function("ingot_processor_single", |b| {
        b.iter(|| {
            let result = processor.process_ingot(black_box(ingot.clone()));
            black_box(result);
        });
    });
}

fn bench_ingot_processor_1000(c: &mut Criterion) {
    let (tx, _rx) = mpsc::channel(1000);
    let metrics = Arc::new(MintMetrics::new());
    let processor = IngotProcessor::new(tx, Arc::clone(&metrics));

    c.bench_function("ingot_processor_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let ingot = create_test_ingot(&format!("test-{}", i), "contract-1");
                let _ = processor.process_ingot(black_box(ingot));
            }
        });
    });
}

fn bench_batcher_add_certificate(c: &mut Criterion) {
    let (tx, _rx) = mpsc::channel(10);
    let metrics = Arc::new(MintMetrics::new());
    let batcher = Batcher::new(tx, 60, Arc::clone(&metrics));
    let cert = create_test_certificate("cert-1", "contract-1");

    c.bench_function("batcher_add_certificate", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = batcher.add_certificate(black_box(cert.clone())).await;
                black_box(result);
            });
        });
    });
}

fn bench_batcher_create_batch(c: &mut Criterion) {
    let (tx, _rx) = mpsc::channel(10);
    let metrics = Arc::new(MintMetrics::new());
    let batcher = Batcher::new(tx, 60, Arc::clone(&metrics));

    let certs: Vec<RoboTorqCertificate> = (0..100).map(|i| {
        create_test_certificate(&format!("cert-{}", i), "contract-1")
    }).collect();

    c.bench_function("batcher_create_batch_100", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = batcher.create_batch(black_box(certs.clone())).await;
                black_box(result);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_merkle_root_single,
    bench_merkle_root_1000,
    bench_merkle_root_10000,
    bench_ingot_processor_single,
    bench_ingot_processor_1000,
    bench_batcher_add_certificate,
    bench_batcher_create_batch
);
criterion_main!(benches);