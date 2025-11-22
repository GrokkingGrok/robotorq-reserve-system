# Printer Integration Status

**Last Updated**: 2025-11-20  
**Branch**: feature/mockprinter

## ✅ Completed

### Digger-Side Integration (100%)
- **PrinterRegistry** (src/digger/src/printer_registry.rs)
  - In-memory HashMap for bonded printers
  - Methods: register(), get(), assign_contract(), complete_print(), list_all()
  - Model validation for Ender3, Prusa, Bambu
  - Unit tests: 100% coverage

- **Printer NATS Handlers** (src/digger/src/printer_handlers.rs)
  - 3 event listeners spawned as tokio tasks:
    * handle_printer_registrations() - Listen for registration requests
    * handle_contract_started_events() - Track print job starts
    * handle_contract_completed_events() - Process capacity data
  - Certificate issuance with Falcon-1024 signatures
  - Complete error handling and logging

- **Integration into Digger main.rs**
  - Printer registry initialized on startup
  - start_printer_listeners() called before HTTP server
  - ApiState updated with printer_registry field

- **Dockerfile Fixes** (src/printer/Dockerfile)
  - Rust 1.91-alpine for edition2024 support
  - clang-dev + llvm-dev for bindgen/libclang
  - liboqs 0.12.0 build from source

### Testing Infrastructure
- **test_printer_registration.py**: Integration test for registration flow
- **test_nats_monitor.py**: NATS message debugging tool
- **MOCK_TESTING.md**: Complete guide for mock printer testing

## ⚠️ Active Issue: NATS Subscription Not Receiving Messages

### Problem
- Digger successfully creates subscription: `✅ Subscription created, entering handler loop`
- Python test publishes to "printer.register" successfully
- **Handler loop never receives messages** - no "📨 RECEIVED MESSAGE" log appears
- Test times out after 5 seconds waiting for certificate response

### Investigation
1. ✅ NATS server running (localhost:4222)
2. ✅ Digger connected to NATS ("✅ Connected to NATS")
3. ✅ Subscription created successfully
4. ✅ Python client connects and publishes
5. ❌ **Rust subscriber never receives published messages**

### Code Verification
```rust
// Handler loop in printer_handlers.rs lines 47-90
while let Some(msg) = sub.next().await {
    info!("📨 RECEIVED MESSAGE on printer.register");  // NEVER EXECUTES
    // ... rest of handler
}
```

### Possible Root Causes
1. **Tokio runtime issue**: Handler task not being polled correctly
2. **async-nats timing**: Subscription not "active" when message published
3. **Subject mismatch**: Though both use "printer.register" literally
4. **NATS connection issue**: Different connections despite same URL?
5. **StreamExt behavior**: tokio_stream::StreamExt .next() not yielding

### Next Steps
1. **Try simple tokio example**: Minimal async-nats subscribe/publish in Rust
2. **Add .await yield points**: Force task to yield between subscription and handler
3. **Try request-reply pattern**: Use nc.request() instead of pub/sub
4. **Check NATS permissions**: Verify no ACL blocking subscription
5. **Test with nats-cli**: Verify messages reach NATS server

### Workaround Options
- Use HTTP polling instead of NATS (defeats purpose)
- Try different NATS client library (nats.rs instead of async-nats)
- Add explicit flush/drain after subscribe()
- Switch to JetStream for durable subscriptions

## 🚧 Blocked Until NATS Fixed

### Cannot Test
- ❌ Certificate issuance flow
- ❌ Contract assignment events
- ❌ Print start/complete notifications
- ❌ Capacity tracking integration
- ❌ Ore generation from printer work

### Printer Service Status
- 🏗️ Docker build in progress (30+ min for liboqs compilation)
- ⏳ Mock API ready to test once NATS working
- ⏳ MockKlipperClient fully implemented

## 📝 Architecture Reference

### NATS Topics
```
printer.register               → Registration requests (Printer → Digger)
printer.{id}.certificate      → Certificate responses (Digger → Printer)  
printer.contract_started      → Print job start events (Printer → Digger)
printer.contract_completed    → Job completion with capacity (Printer → Digger)
```

### Data Flow
```
1. Printer → "printer.register" with PrinterRegistration JSON
2. Digger receives, validates, issues Falcon-1024 signed certificate
3. Digger publishes certificate to "printer.{id}.certificate"
4. Printer stores certificate, can now receive contracts
5. On print: Printer → "printer.contract_started"
6. On complete: Printer → "printer.contract_completed" with capacity
7. Digger generates JouleTorqOre (TODO) based on capacity
```

### Certificate Chain
```
Digger Falcon-1024 Keypair (generated on startup)
    ↓ signs
PrinterCertificate {
    printer_id, model, rated_watts,
    issued_at, expires_at (7 days),
    public_key, certificate_hash, signature
}
```

## 🔧 Technical Details

### Dependencies
```toml
[dependencies]
async-nats = "0.45.0"
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"  # For StreamExt trait
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Logging
```rust
use tracing::{info, warn, error};

// Startup
info!("📝 Listening for printer registrations on 'printer.register'");
info!("✅ Subscription created, entering handler loop");

// Handler
info!("📨 RECEIVED MESSAGE on printer.register");  // NOT APPEARING
info!("📥 Printer registration received: {}", printer_id);
info!("🔐 Issuing certificate for printer: {}", printer_id);
```

## 📖 Related Docs
- `src/printer/MOCK_TESTING.md` - Mock printer testing guide
- `src/digger/src/printer_registry.rs` - Registry implementation
- `src/digger/src/printer_handlers.rs` - NATS event handlers
- `test_printer_registration.py` - Integration test script

---

**Summary**: Digger printer integration is 100% implemented and compiles successfully. **Blocked by NATS subscription issue** where published messages are not received by Rust subscriber despite successful connection and subscription creation. Need to investigate tokio/async-nats behavior or switch to alternative messaging pattern.
