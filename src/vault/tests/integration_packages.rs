use robotorq_vault::models::{UBDDistributionPackage, DemurrageReleasePackage, PackageConfirmation, PackageType};
use robotorq_vault::events::subjects;
use async_nats::connect;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::StreamExt;

#[tokio::test]
async fn test_ubd_package_delivery_flow() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("NATS server required");

    // Create a test package
    let package = UBDDistributionPackage::new(
        "test-user-123".to_string(),
        1000,
        vec!["cert-1".to_string(), "cert-2".to_string()],
    );

    // Subscribe to package delivery
    let received_packages = Arc::new(Mutex::new(Vec::new()));
    let received_packages_clone = received_packages.clone();

    let mut subscriber = nats.subscribe(subjects::VAULT_UBD_PACKAGE).await.unwrap();
    tokio::spawn(async move {
        while let Some(message) = subscriber.next().await {
            let package: UBDDistributionPackage = serde_json::from_slice(&message.payload).unwrap();
            received_packages_clone.lock().await.push(package);
        }
    });

    // Publish package
    let payload = serde_json::to_vec(&package).unwrap();
    nats.publish(subjects::VAULT_UBD_PACKAGE, payload.into()).await.unwrap();

    // Wait for delivery
    sleep(Duration::from_millis(100)).await;

    // Verify reception
    let packages = received_packages.lock().await;
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].user_id, "test-user-123");
    assert_eq!(packages[0].amount_canonical_jouletorq, 1000);
    assert_eq!(packages[0].package_hash, package.package_hash);
}

#[tokio::test]
async fn test_demurrage_package_delivery_flow() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("NATS server required");

    // Create a test package
    let package = DemurrageReleasePackage::new("test-user-456".to_string(), 500);

    // Subscribe to package delivery
    let received_packages = Arc::new(Mutex::new(Vec::new()));
    let received_packages_clone = received_packages.clone();

    let mut subscriber = nats.subscribe(subjects::VAULT_DEMURRAGE_PACKAGE).await.unwrap();
    tokio::spawn(async move {
        while let Some(message) = subscriber.next().await {
            let package: DemurrageReleasePackage = serde_json::from_slice(&message.payload).unwrap();
            // Only collect packages for our test user
            if package.user_id == "test-user-456" {
                received_packages_clone.lock().await.push(package);
            }
        }
    });

    // Publish package
    let payload = serde_json::to_vec(&package).unwrap();
    nats.publish(subjects::VAULT_DEMURRAGE_PACKAGE, payload.into()).await.unwrap();

    // Wait for delivery
    sleep(Duration::from_millis(100)).await;

    // Verify reception
    let packages = received_packages.lock().await;
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].user_id, "test-user-456");
    assert_eq!(packages[0].amount_canonical_jouletorq, 500);
    assert_eq!(packages[0].package_hash, package.package_hash);
}

#[tokio::test]
async fn test_package_confirmation_flow() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("NATS server required");

    // Create a test confirmation
    let confirmation = PackageConfirmation::new(
        "pkg-123".to_string(),
        "test-user-789".to_string(),
        PackageType::UBD,
        1000,
        "hash123".to_string(),
    );

    // Subscribe to confirmations
    let received_confirmations = Arc::new(Mutex::new(Vec::new()));
    let received_confirmations_clone = received_confirmations.clone();

    let mut subscriber = nats.subscribe(subjects::WALLET_PACKAGE_CONFIRMATION).await.unwrap();
    tokio::spawn(async move {
        while let Some(message) = subscriber.next().await {
            let confirmation: PackageConfirmation = serde_json::from_slice(&message.payload).unwrap();
            // Only collect confirmations for our test package
            if confirmation.package_id == "pkg-123" {
                received_confirmations_clone.lock().await.push(confirmation);
            }
        }
    });

    // Publish confirmation
    let payload = serde_json::to_vec(&confirmation).unwrap();
    nats.publish(subjects::WALLET_PACKAGE_CONFIRMATION, payload.into()).await.unwrap();

    // Wait for delivery
    sleep(Duration::from_millis(100)).await;

    // Verify reception
    let confirmations = received_confirmations.lock().await;
    assert_eq!(confirmations.len(), 1);
    assert_eq!(confirmations[0].package_id, "pkg-123");
    assert_eq!(confirmations[0].user_id, "test-user-789");
    assert_eq!(confirmations[0].received_amount, 1000);
    assert_eq!(confirmations[0].package_hash, "hash123");
}

#[tokio::test]
async fn test_package_hash_verification() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("NATS server required");

    // Create original package
    let original_package = UBDDistributionPackage::new(
        "test-user".to_string(),
        1000,
        vec!["cert-1".to_string()],
    );

    // Simulate wallet receiving and confirming with correct hash
    let correct_confirmation = PackageConfirmation::new(
        original_package.package_id.clone(),
        original_package.user_id.clone(),
        PackageType::UBD,
        original_package.amount_canonical_jouletorq,
        original_package.package_hash.clone(),
    );

    // Simulate wallet receiving and confirming with wrong hash
    let wrong_confirmation = PackageConfirmation::new(
        original_package.package_id.clone(),
        original_package.user_id.clone(),
        PackageType::UBD,
        original_package.amount_canonical_jouletorq,
        "wrong-hash".to_string(),
    );

    // Subscribe to confirmations
    let received_confirmations = Arc::new(Mutex::new(Vec::new()));
    let received_confirmations_clone = received_confirmations.clone();

    let mut subscriber = nats.subscribe(subjects::WALLET_PACKAGE_CONFIRMATION).await.unwrap();
    tokio::spawn(async move {
        while let Some(message) = subscriber.next().await {
            let confirmation: PackageConfirmation = serde_json::from_slice(&message.payload).unwrap();
            // Only collect confirmations for our test package
            if confirmation.package_id == original_package.package_id {
                received_confirmations_clone.lock().await.push(confirmation);
            }
        }
    });

    // Publish both confirmations
    nats.publish(subjects::WALLET_PACKAGE_CONFIRMATION,
                serde_json::to_vec(&correct_confirmation).unwrap().into()).await.unwrap();
    nats.publish(subjects::WALLET_PACKAGE_CONFIRMATION,
                serde_json::to_vec(&wrong_confirmation).unwrap().into()).await.unwrap();

    // Wait for delivery
    sleep(Duration::from_millis(100)).await;

    // Verify both were received
    let confirmations = received_confirmations.lock().await;
    assert_eq!(confirmations.len(), 2);

    // Check that one has correct hash, one has wrong hash
    let correct_count = confirmations.iter()
        .filter(|c| c.package_hash == original_package.package_hash)
        .count();
    let wrong_count = confirmations.iter()
        .filter(|c| c.package_hash == "wrong-hash")
        .count();

    assert_eq!(correct_count, 1);
    assert_eq!(wrong_count, 1);
}

#[tokio::test]
async fn test_multiple_package_types() {
    let nats = connect("nats://127.0.0.1:4222").await.expect("NATS server required");

    // Create different package types
    let ubd_package = UBDDistributionPackage::new(
        "user-1".to_string(),
        1000,
        vec!["cert-1".to_string()],
    );
    let demurrage_package = DemurrageReleasePackage::new("user-2".to_string(), 500);

    // Subscribe to both package types
    let received_ubd = Arc::new(Mutex::new(Vec::new()));
    let received_demurrage = Arc::new(Mutex::new(Vec::new()));

    let ubd_clone = received_ubd.clone();
    let demurrage_clone = received_demurrage.clone();

    let mut ubd_subscriber = nats.subscribe(subjects::VAULT_UBD_PACKAGE).await.unwrap();
    let mut demurrage_subscriber = nats.subscribe(subjects::VAULT_DEMURRAGE_PACKAGE).await.unwrap();

    tokio::spawn(async move {
        while let Some(message) = ubd_subscriber.next().await {
            let package: UBDDistributionPackage = serde_json::from_slice(&message.payload).unwrap();
            // Only collect packages for our test user
            if package.user_id == "user-1" {
                ubd_clone.lock().await.push(package);
            }
        }
    });

    tokio::spawn(async move {
        while let Some(message) = demurrage_subscriber.next().await {
            let package: DemurrageReleasePackage = serde_json::from_slice(&message.payload).unwrap();
            // Only collect packages for our test user
            if package.user_id == "user-2" {
                demurrage_clone.lock().await.push(package);
            }
        }
    });

    // Publish both types
    nats.publish(subjects::VAULT_UBD_PACKAGE,
                serde_json::to_vec(&ubd_package).unwrap().into()).await.unwrap();
    nats.publish(subjects::VAULT_DEMURRAGE_PACKAGE,
                serde_json::to_vec(&demurrage_package).unwrap().into()).await.unwrap();

    // Wait for delivery
    sleep(Duration::from_millis(100)).await;

    // Verify both were received on correct subjects
    assert_eq!(received_ubd.lock().await.len(), 1);
    assert_eq!(received_demurrage.lock().await.len(), 1);
}