use criterion::{black_box, criterion_group, criterion_main, Criterion};
use robotorq_vault::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation, PackageType};
use robotorq_vault::{ShortVault, ShortVaultRegistry};
use async_nats::Client;
use std::sync::Arc;

// Mock NATS client for benchmarks (simplified)
fn mock_nats_client() -> Client {
    // In a real benchmark, you'd set up a test NATS server
    // For now, we'll skip NATS-dependent benchmarks
    unimplemented!("Mock NATS client needed for benchmarks")
}

fn bench_package_creation(c: &mut Criterion) {
    c.bench_function("ubd_package_creation", |b| {
        b.iter(|| {
            let package = UBDDistributionPackage::new(
                black_box("bench-user-123".to_string()),
                black_box(1000),
                black_box(vec!["cert-1".to_string(), "cert-2".to_string()]),
            );
            black_box(package);
        });
    });

    c.bench_function("demurrage_package_creation", |b| {
        b.iter(|| {
            let package = DemurrageReleasePackage::new(
                black_box("bench-user-456".to_string()),
                black_box(500),
            );
            black_box(package);
        });
    });

    c.bench_function("package_confirmation_creation", |b| {
        b.iter(|| {
            let confirmation = PackageConfirmation::new(
                black_box("pkg-123".to_string()),
                black_box("bench-user-789".to_string()),
                black_box(PackageType::UBD),
                black_box(1000),
                black_box("hash123".to_string()),
            );
            black_box(confirmation);
        });
    });
}

fn bench_package_serialization(c: &mut Criterion) {
    let ubd_package = UBDDistributionPackage::new(
        "bench-user".to_string(),
        1000,
        vec!["cert-1".to_string()],
    );

    let demurrage_package = DemurrageReleasePackage::new("bench-user".to_string(), 500);

    let confirmation = PackageConfirmation::new(
        "pkg-123".to_string(),
        "bench-user".to_string(),
        PackageType::UBD,
        1000,
        "hash123".to_string(),
    );

    c.bench_function("ubd_package_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&ubd_package)).unwrap();
            black_box(json);
        });
    });

    c.bench_function("ubd_package_deserialize", |b| {
        let json = serde_json::to_string(&ubd_package).unwrap();
        b.iter(|| {
            let package: UBDDistributionPackage = serde_json::from_str(black_box(&json)).unwrap();
            black_box(package);
        });
    });

    c.bench_function("demurrage_package_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&demurrage_package)).unwrap();
            black_box(json);
        });
    });

    c.bench_function("confirmation_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&confirmation)).unwrap();
            black_box(json);
        });
    });
}

fn bench_short_vault_operations(c: &mut Criterion) {
    // Note: These benchmarks would require a real NATS setup
    // For now, we'll benchmark the computational parts

    c.bench_function("drip_schedule_calculation", |b| {
        let total_amount = 1000i64;
        let duration_hours = 2.0f64;

        b.iter(|| {
            let drip_rate = black_box(total_amount) as f64 / black_box(duration_hours);
            black_box(drip_rate);
        });
    });

    c.bench_function("balance_calculations", |b| {
        let balance = 5000i64;
        let minimum = 1000i64;

        b.iter(|| {
            let available = balance.saturating_sub(minimum);
            black_box(available);
        });
    });
}

fn bench_registry_operations(c: &mut Criterion) {
    c.bench_function("registry_balance_aggregation", |b| {
        let balances = vec![1000i64, 2000, 3000, 4000, 5000];

        b.iter(|| {
            let total: i64 = balances.iter().sum();
            black_box(total);
        });
    });
}

criterion_group!(
    benches,
    bench_package_creation,
    bench_package_serialization,
    bench_short_vault_operations,
    bench_registry_operations
);
criterion_main!(benches);