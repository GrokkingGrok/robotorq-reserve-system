# Printer Service - Demo Implementation Plan

**Goal**: Minimal working demo showing capacity-based pricing with physical printer integration.

## Phase 1: Demo Scope (What We're Building NOW)

### Flow Overview

```
1. Printer boots → Registers with Digger
   ├─> Digger validates model specs (Ender 3 V3 KE = 350W)
   └─> Digger signs simple certificate (no Mint, no manufacturer)

2. Digger assigns contract to printer
   ├─> NATS: printer.{id}.contract_assigned
   └─> Printer stores: current_contract_id

3. User presses PRINT on physical printer
   ├─> Printer detects: Klipper state = "printing"
   ├─> NATS: printer.contract_started
   │   └─> { contract_id, printer_id, timestamp }
   └─> Digger starts timer

4. Print completes (or cancelled/paused)
   ├─> Printer detects: Klipper state = "complete"
   ├─> Calculate duration = now() - start_time
   ├─> Calculate capacity = 350W × duration_hours
   ├─> NATS: printer.contract_completed
   │   └─> { contract_id, duration_hours, capacity_kwh }
   └─> Digger generates JouleTorqOre
```

---

## What's IN Phase 1 ✅

### Printer Service (Rust)
- ✅ Klipper/Moonraker integration (query print state)
- ✅ Registration with Digger
- ✅ Certificate storage (Digger-signed, not Mint)
- ✅ State detection (idle → printing → idle)
- ✅ Duration tracking
- ✅ NATS messages:
  - `printer.register`
  - `printer.status` (heartbeat)
  - `printer.contract_started`
  - `printer.contract_completed`

### Digger Service (Rust)
- ✅ NATS subscribe `printer.register`
- ✅ Validate printer model (hardcoded DB: Ender3V3KE = 350W)
- ✅ Issue simple certificate (Digger signs with Falcon-1024)
- ✅ NATS subscribe `printer.contract_started`
- ✅ NATS subscribe `printer.contract_completed`
- ✅ Track printer state in memory
- ✅ Calculate capacity-hours
- ✅ Generate JouleTorqOre with capacity value

### Demo Flow
1. Start services: `docker-compose up -d`
2. Start Printer service on local machine
3. Printer registers → receives certificate
4. Create contract in Digger: `curl -X POST http://localhost:9000/contract/create`
5. Assign contract to printer: `curl -X POST http://localhost:9000/contract/{id}/assign_printer`
6. Press PRINT on Ender 3 V3 KE
7. Watch Digger logs: "Print started for contract X"
8. Wait for print to finish
9. Watch Digger logs: "Print completed: 2.5 hours, 0.875 kWh capacity"
10. Check Refinery: Ore received with capacity value

---

## What's OUT of Phase 1 ❌

### Mint Certificate Signing
- ❌ No SPHINCS+ signatures (Falcon-1024 only)
- ❌ No Mint `/issue_printer_certificate` endpoint
- ❌ No ProofCache storage
- **Why defer**: Demo doesn't need Mint authority yet

### Manufacturer Attestation
- ❌ No serial number verification
- ❌ No manufacturer API calls
- ❌ No manufacturer certificate chain
- **Why defer**: Manual trust for demo (hardcoded specs)

### Certificate Expiration/Renewal
- ❌ No expiration checks
- ❌ No renewal flow
- **Why defer**: Demo is short-lived

### Job Queue (Digger → Printer)
- ❌ Printer doesn't accept G-code from network
- ❌ User starts print manually on printer
- **Why defer**: Focus on capacity tracking first

### Advanced Features
- ❌ Milestone reporting (layer progress)
- ❌ Multi-printer support
- ❌ Certificate revocation
- ❌ Warranty integration

---

## Data Structures

### Printer Certificate (Demo Version)
```json
{
  "certificate_id": "cert-printer-abc123",
  "printer_id": "ender3-v3-ke-001",
  "printer_model": "Ender3V3KE",
  "rated_watts": 350,
  "issued_by": "digger-001",
  "issued_at": "2025-11-19T10:00:00Z",
  "valid_until": "2026-11-19T10:00:00Z",
  "digger_signature": "falcon1024_signature_hex",
  "digger_public_key": "falcon1024_pubkey_hex"
}
```

### Contract Assignment
```json
{
  "contract_id": "demo-contract-001",
  "printer_id": "ender3-v3-ke-001",
  "assigned_at": "2025-11-19T10:05:00Z",
  "status": "assigned"
}
```

### Print Start Event
```json
{
  "event_type": "contract_started",
  "contract_id": "demo-contract-001",
  "printer_id": "ender3-v3-ke-001",
  "rated_watts": 350,
  "timestamp": "2025-11-19T10:10:00Z",
  "signature": "printer_falcon_signature"
}
```

### Print Completion Event
```json
{
  "event_type": "contract_completed",
  "contract_id": "demo-contract-001",
  "printer_id": "ender3-v3-ke-001",
  "duration_hours": 2.5,
  "capacity_kwh": 0.875,
  "rated_watts": 350,
  "timestamp": "2025-11-19T12:40:00Z",
  "signature": "printer_falcon_signature"
}
```

### JouleTorqOre (With Capacity)
```json
{
  "ore_id": "ore-20251119-001",
  "contract_id": "demo-contract-001",
  "printer_id": "ender3-v3-ke-001",
  "capacity_kwh": 0.875,
  "duration_hours": 2.5,
  "rated_watts": 350,
  "robo_stake": 0.000875,
  "timestamp": "2025-11-19T12:40:05Z"
}
```

---

## Digger Changes Needed

### 1. Printer Registry (In-Memory)
```rust
struct PrinterRegistry {
    printers: HashMap<String, BondedPrinter>,
}

struct BondedPrinter {
    printer_id: String,
    model: String,
    rated_watts: u32,
    certificate: Certificate,
    current_contract_id: Option<String>,
    status: PrinterStatus,  // Idle, Printing, Error
    last_seen: DateTime<Utc>,
}

enum PrinterStatus {
    Idle,
    Printing { started_at: DateTime<Utc> },
    Error,
}
```

### 2. NATS Handlers
```rust
// Subscribe to printer.register
async fn handle_printer_registration(msg: Message) {
    let reg: PrinterRegistration = serde_json::from_slice(&msg.data)?;
    
    // Validate model
    if !validate_printer_model(&reg.printer_model, reg.rated_watts) {
        return Err("Invalid model specs");
    }
    
    // Issue certificate
    let cert = issue_printer_certificate(&reg)?;
    
    // Store in registry
    printer_registry.insert(reg.printer_id, BondedPrinter {
        printer_id: reg.printer_id.clone(),
        model: reg.printer_model,
        rated_watts: reg.rated_watts,
        certificate: cert.clone(),
        current_contract_id: None,
        status: PrinterStatus::Idle,
        last_seen: Utc::now(),
    });
    
    // Send certificate back
    nats_client.publish(
        format!("printer.{}.certificate", reg.printer_id),
        serde_json::to_vec(&cert)?
    ).await?;
}

// Subscribe to printer.contract_started
async fn handle_contract_started(msg: Message) {
    let event: ContractStartEvent = serde_json::from_slice(&msg.data)?;
    
    // Update printer status
    if let Some(printer) = printer_registry.get_mut(&event.printer_id) {
        printer.status = PrinterStatus::Printing {
            started_at: event.timestamp,
        };
    }
    
    // Update contract
    if let Some(contract) = contract_manager.get_mut(&event.contract_id) {
        contract.start_time = Some(event.timestamp);
        contract.status = ContractStatus::InProgress;
    }
    
    info!("🖨️  Print started: contract={} printer={}", 
          event.contract_id, event.printer_id);
}

// Subscribe to printer.contract_completed
async fn handle_contract_completed(msg: Message) {
    let event: ContractCompletionEvent = serde_json::from_slice(&msg.data)?;
    
    // Update printer status
    if let Some(printer) = printer_registry.get_mut(&event.printer_id) {
        printer.status = PrinterStatus::Idle;
        printer.current_contract_id = None;
    }
    
    // Calculate RoboStake
    let robo_stake = event.capacity_kwh * RT_RATE;
    
    // Generate ore
    let ore = JouleTorqOre {
        ore_id: generate_ore_id(),
        contract_id: event.contract_id.clone(),
        printer_id: event.printer_id.clone(),
        capacity_kwh: event.capacity_kwh,
        duration_hours: event.duration_hours,
        rated_watts: event.rated_watts,
        robo_stake,
        timestamp: Utc::now(),
    };
    
    // Send to Refinery
    send_ore_to_refinery(&ore).await?;
    
    info!("✅ Print completed: {}hrs, {:.4}kWh, {:.6}RT",
          event.duration_hours, event.capacity_kwh, robo_stake);
}
```

### 3. Model Validation Database
```rust
const SUPPORTED_PRINTERS: &[(&str, u32)] = &[
    ("Ender3V3KE", 350),
    ("PrusaMK4", 270),
    ("BambuX1Carbon", 350),
];

fn validate_printer_model(model: &str, rated_watts: u32) -> bool {
    SUPPORTED_PRINTERS.iter()
        .any(|(m, w)| *m == model && *w == rated_watts)
}
```

### 4. Contract Assignment API
```rust
// POST /contract/{id}/assign_printer
async fn assign_printer_to_contract(
    contract_id: String,
    printer_id: String,
) -> Result<()> {
    // Validate printer is registered
    let printer = printer_registry.get(&printer_id)
        .ok_or("Printer not registered")?;
    
    // Validate printer is idle
    if !matches!(printer.status, PrinterStatus::Idle) {
        return Err("Printer is busy");
    }
    
    // Update contract
    let contract = contract_manager.get_mut(&contract_id)
        .ok_or("Contract not found")?;
    contract.assigned_printer_id = Some(printer_id.clone());
    
    // Update printer
    printer_registry.get_mut(&printer_id).unwrap()
        .current_contract_id = Some(contract_id.clone());
    
    // Notify printer
    nats_client.publish(
        format!("printer.{}.contract_assigned", printer_id),
        serde_json::to_vec(&ContractAssignment {
            contract_id,
            printer_id,
            assigned_at: Utc::now(),
        })?
    ).await?;
    
    Ok(())
}
```

---

## Testing the Demo

### 1. Start Infrastructure
```bash
docker-compose up -d nats refinery mint
```

### 2. Start Digger (with printer support)
```bash
cd src/digger
cargo run
```

### 3. Start Printer Service
```bash
cd src/printer
# Edit config.yaml with your printer IP
cargo run
```

### 4. Register Printer (Automatic)
```
Printer → Sends registration
Digger → Validates Ender3V3KE = 350W
Digger → Issues certificate
Printer → Saves certificate
```

### 5. Create & Assign Contract
```bash
# Create contract
CONTRACT_ID=$(curl -s -X POST http://localhost:9000/contract/create \
  -H "Content-Type: application/json" \
  -d '{"description":"Demo print job"}' | jq -r '.contract_id')

# Assign to printer
curl -X POST http://localhost:9000/contract/$CONTRACT_ID/assign_printer \
  -H "Content-Type: application/json" \
  -d '{"printer_id":"ender3-v3-ke-001"}'
```

### 6. Start Print Job
Press PRINT on your Ender 3 V3 KE

**Expected Logs**:
```
Digger: 🖨️  Print started: contract=demo-contract-001 printer=ender3-v3-ke-001
Digger: Timer started for capacity tracking
```

### 7. Wait for Completion
**Expected Logs**:
```
Digger: ✅ Print completed: 2.5hrs, 0.8750kWh, 0.000875RT
Digger: Generated ore: ore-20251119-001
Refinery: Received ore from printer ender3-v3-ke-001
```

---

## Success Criteria

- ✅ Printer registers and receives certificate
- ✅ Contract assignment works
- ✅ Print start detected by Digger
- ✅ Duration calculated correctly
- ✅ Capacity = rated_watts × duration
- ✅ Ore generated with capacity value
- ✅ Refinery receives ore

---

## Next Steps After Demo

### Phase 2: Mint Integration
- Digger forwards cert requests to Mint
- Mint signs with SPHINCS+
- ProofCache storage

### Phase 3: Manufacturer Attestation
- Serial number verification
- Manufacturer API integration
- Certificate chain of trust

### Phase 4: Job Queue
- Digger sends G-code to printer
- Printer executes jobs from network
- Milestone reporting

### Phase 5: Economic Integration
- RoboStake pricing
- DistoDam distribution
- Wallet balance tracking
