# Digger Architecture & Documentation

## Overview

The **Digger** is a Tauri-based desktop application that manages mining contracts in the RoboTorq economic system. It generates JouleTorqOre (digital energy + tokens + RoboStake) and sends it to the Refinery for processing into TokenTorqIngots.

## Table of Contents

1. [Economic Model](#economic-model)
2. [Contract Flow](#contract-flow)
3. [API Endpoints](#api-endpoints)
4. [Dashboard UI](#dashboard-ui)
5. [State Management](#state-management)
6. [Refinery Integration](#refinery-integration)
7. [Testing](#testing)

---

## Economic Model

### Core Concepts

**RoboStake (RT)**: The economic fuel that powers mining contracts. Provided by the Trust service.

**Contract Duration**: Calculated based on RoboStake amount and digger capabilities:
```
duration_hours = amount_rt / (power_kw × max_token_throughput)
```

**Milestones**: Discrete work intervals (default: 5 seconds) where the digger produces ore.

**Total Milestones**: 
```
total_milestones = (duration_hours × 3600) / interval_seconds
```

**RoboStake per Milestone**:
```
robo_per_milestone = robo_stake_total / total_milestones
```

### Example Calculation

Given:
- `amount_rt` = 100.0 RT
- `power_kw` = 0.25 kW  
- `max_token_throughput` = 12 tokens/sec
- `interval_seconds` = 5 seconds

Calculations:
```
duration_hours = 100 / (0.25 × 12) = 33.33 hours
total_milestones = (33.33 × 3600) / 5 = 24,000 milestones
robo_per_milestone = 100 / 24,000 = 0.004167 RT
```

### Ore Generation

Each milestone produces:
- **Tokens**: `max_token_throughput × interval_seconds × torq` 
  - Example: 12 × 5 × 1 = **60 tokens**
- **Joules**: `power_kw × 1000 × interval_seconds`
  - Example: 0.25 × 1000 × 5 = **1,250 joules**
- **RoboStake**: `robo_per_milestone`
  - Example: **0.004167 RT**

---

## Contract Flow

### 1. Stake Reception (POST /stake)

**Endpoint**: `http://localhost:9000/stake`

**Request**:
```json
{
  "contract_id": "test-contract-1",
  "amount_rt": 100.0
}
```

**Response**:
```json
{
  "status": "accepted",
  "contract_id": "test-contract-1", 
  "message": "Received 100 RT for contract, duration: 33.33 hours"
}
```

**Processing**:
1. Validates JSON payload
2. Looks up digger by ID (currently hardcoded to `dig-jon-ai-001`)
3. Calculates contract duration
4. Updates or creates contract with stake and duration
5. Stores contract in digger's `current_contract`

### 2. Contract Initialization

**Tauri Command**: `start_contract(digger_id, contract_id)`

**Process**:
1. Retrieves contract from digger
2. Validates contract is authorized
3. Creates session ID: `{digger_id}-{contract_id}`
4. Calculates end time: `start_time + (duration_hours × 3600)`
5. Spawns async execution task
6. Emits `contract_started` event to UI

### 3. Contract Execution Loop

**Lifecycle**:
```rust
loop {
    // Check timer
    if current_time >= contract_end_time {
        emit("contract_completed");
        break;
    }
    
    // Check state (pause/resume/stop)
    match state_manager.get_state(contract_id) {
        ContractControl::Paused => {
            sleep(1s);
            continue;
        }
        ContractControl::Stopped => {
            emit("contract_stopped");
            break;
        }
        _ => {}
    }
    
    // Generate ore
    let ore = create_ore(milestone_index, robo_per_milestone);
    
    // Send to Refinery
    match send_ore_to_refinery(&ore).await {
        Ok(()) => mark_milestone_confirmed(),
        Err(e) => mark_milestone_failed(e),
    }
    
    // Update totals
    total_tokens += ore.tokens_generated;
    total_joules += ore.joules;
    robo_stake_sent += ore.robo_stake_amount;
    
    // Emit status update
    emit("contract_status_update", status);
    
    // Sleep until next milestone
    sleep(interval_seconds);
    milestone_index += 1;
}
```

### 4. Contract Completion

**Timer-Based Completion**:
- When `SystemTime::now() >= contract_end_time`
- Emits `contract_completed` event
- Dashboard shows final statistics

**Manual Stop**:
- User clicks "Stop Contract" button
- Emits `contract_stopped` event
- Contract state set to `ContractControl::Stopped`

---

## API Endpoints

### HTTP API (Port 9000)

#### POST /stake
**Purpose**: Receive RoboStake from Trust service

**Request Body**:
```json
{
  "contract_id": "string",
  "amount_rt": number
}
```

**Response** (200 OK):
```json
{
  "status": "accepted",
  "contract_id": "string",
  "message": "Received X RT for contract, duration: Y hours"
}
```

**Response** (404 Not Found):
```json
"Digger not found"
```

#### GET /robot/status
**Purpose**: Query current robot status

**Response**:
```json
{
  "robot_id": "dig-jon-ai-001",
  "current_contract": "contract-001" | null
}
```

#### GET /health
**Purpose**: Health check endpoint

**Response**: `"ok"`

### Tauri Commands (Frontend → Backend)

#### start_contract
```rust
start_contract(digger_id: String, contract_id: String) -> Result<String, String>
```

**Success**: `"Robot_{id}_started_contract_{id}"`  
**Error**: `"Contract not found"`, `"Robot not found"`, `"Contract not approved"`

#### pause_contract
```rust
pause_contract(contract_id: String, app: AppHandle) -> Result<String, String>
```

**Effect**: Sets state to `ContractControl::Paused`, emits `contract_paused` event

#### resume_contract
```rust
resume_contract(contract_id: String, app: AppHandle) -> Result<String, String>
```

**Effect**: Sets state to `ContractControl::Running`, emits `contract_resumed` event

#### stop_contract
```rust
stop_contract(contract_id: String, app: AppHandle) -> Result<String, String>
```

**Effect**: Sets state to `ContractControl::Stopped`, triggers shutdown

#### get_milestone_statuses
```rust
get_milestone_statuses(contract_id: String) -> Result<serde_json::Value, String>
```

**Response**:
```json
{
  "total": 24000,
  "confirmed": 150,
  "failed": 2,
  "pending": 23848
}
```

---

## Dashboard UI

### Economics Panel
**Metrics**:
- **Total Tokens**: Cumulative tokens generated
- **Total Joules**: Cumulative energy consumed
- **RoboStake Received**: Initial stake amount from Trust
- **RoboStake Sent**: Cumulative RT sent to Refinery

**Update Frequency**: Every `interval_seconds` (5s default)

### Progress Panel
**Metrics**:
- **Progress Bar**: Animated gradient (green → cyan)
  - Width: `(current_milestone / total_milestones) × 100%`
- **Current Milestone**: `X / Y`
- **Percent Complete**: `(current / total) × 100%`
- **Time Elapsed**: Formatted as "Xh Ym Zs"
- **Time Remaining**: Formatted as "Xh Ym Zs"

**Visual Feedback**: Progress bar fills from left to right

### Health Panel
**Indicators**:
- **Refinery Status**: 🟢 Healthy / 🔴 Unhealthy
  - Green if last milestone accepted
  - Red if last milestone failed
- **Milestones Confirmed**: Count of successful Refinery submissions
- **Milestones Failed**: Count of failed Refinery submissions

### Controls Panel
**Buttons**:
- **Pause** (orange): Sets contract to paused state
- **Resume** (green): Resumes paused contract
- **Stop** (red): Terminates contract permanently

**Visibility**:
- Hidden until contract starts
- Pause shown when running
- Resume shown when paused

---

## State Management

### ContractControl Enum

```rust
pub enum ContractControl {
    Running,   // Contract actively executing
    Paused,    // Temporarily suspended (can resume)
    Stopped,   // Permanently terminated (cannot resume)
    Completed, // Finished naturally (timer expired)
}
```

### ContractStateManager (Singleton)

**Purpose**: Global state tracking for all active contracts

**Methods**:
```rust
set_state(contract_id: &str, state: ContractControl)
get_state(contract_id: &str) -> ContractControl
```

**Thread Safety**: `Arc<Mutex<HashMap<String, ContractControl>>>`

### MilestoneStatus Enum

```rust
pub enum MilestoneStatus {
    Pending,           // Not yet sent
    Sending,           // Currently transmitting
    Confirmed,         // Refinery accepted
    Failed(String),    // Refinery rejected (reason stored)
}
```

### MilestoneTracker (Singleton)

**Purpose**: Track individual milestone submission results

**Key**: `{contract_id}:{milestone_index}`

**Methods**:
```rust
set_status(contract_id: &str, milestone: u32, status: MilestoneStatus)
get_status(contract_id: &str, milestone: u32) -> Option<MilestoneStatus>
get_all_for_contract(contract_id: &str) -> (total, confirmed, failed, pending)
```

---

## Refinery Integration

### Async HTTP Client

**Library**: `reqwest` (async, not blocking)

**Configuration**:
- Base URL: `http://localhost:8081` (configurable via `REFINERY_URL` env var)
- Timeout: 5 seconds
- Content-Type: `application/json`

### Ore Submission

**Endpoint**: `POST /receive-ore`

**Request Body**:
```json
{
  "digger_id": "dig-jon-ai-001",
  "contract_id": "test-contract-1",
  "tokens_generated": 60,
  "joules": 1250,
  "milestone_index": 0,
  "timestamp": 1731614400,
  "proof_of_work": null,
  "robo_stake_amount": 0.004167,
  "signature": null
}
```

**Response** (200 OK):
```json
{
  "status": "accepted",
  "contract_id": "test-contract-1",
  "milestone": 0
}
```

**Response** (429 Too Many Requests):
```json
"Joule queue full, retry later"
```

### Error Handling

**Network Errors**:
- Connection refused → Mark milestone as `Failed("Connection refused")`
- Timeout → Mark milestone as `Failed("Request timeout")`

**HTTP Errors**:
- 429 Queue Full → Mark as `Failed("Refinery queue full")`
- 400 Bad Request → Mark as `Failed("Invalid ore data")`

**Success**:
- 200 OK → Mark milestone as `Confirmed`
- Update `robo_stake_sent` running total
- Emit `milestone_confirmed` event

---

## Testing

### Unit Tests (13 tests, all passing)

**Location**: `src-tauri/src/digger.rs` (inline `#[cfg(test)]` module)

**Coverage**:
- `ContractControl` enum behavior
- `MilestoneStatus` enum variants
- `ContractStateManager` state transitions
- `MilestoneTracker` status tracking
- Economic calculations (robo_per_milestone, total_milestones)
- Edge cases (zero stake, multi-contract isolation)

**Run Tests**:
```bash
cd src-tauri
cargo test
```

### Integration Testing

**Test Plan**: See `TESTING_PLAN.md`

**Prerequisites**:
```bash
# Start services
docker-compose up -d nats refinery

# Verify health
docker logs robotorq-network-refinery-1
```

**Test Scenarios**:
1. Basic contract start
2. RoboStake integration (Trust → Digger)
3. Real-time dashboard updates
4. Refinery integration (Digger → Refinery)
5. Milestone status tracking
6. Pause/resume controls
7. Stop contract functionality
8. Timer-based completion
9. Multiple contracts (isolation)
10. Error handling

**Verified Results** (Nov 14, 2025):
- ✅ End-to-end integration functional
- ✅ Ore successfully sent and accepted by Refinery
- ✅ Ingots minting successfully
- ✅ Dashboard updating in real-time
- ✅ No runtime panics (async client working)

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    Digger Desktop App                    │
│                      (Tauri + Rust)                      │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Frontend (HTML/CSS/JS)          Backend (Rust)          │
│  ┌─────────────────────┐         ┌──────────────────┐   │
│  │  Dashboard UI       │◄────────┤  Tauri Commands  │   │
│  │  - Economics Panel  │  Events │  - start_contract│   │
│  │  - Progress Panel   │◄────────┤  - pause/resume  │   │
│  │  - Health Panel     │         │  - stop_contract │   │
│  │  - Controls Panel   │         └──────────────────┘   │
│  └─────────────────────┘                 │               │
│                                           │               │
│                              ┌────────────▼──────────┐   │
│                              │  Contract Execution   │   │
│                              │  - Timer Management   │   │
│                              │  - Ore Generation     │   │
│                              │  - State Tracking     │   │
│                              └────────────┬──────────┘   │
│                                           │               │
│                              ┌────────────▼──────────┐   │
│                              │  HTTP API Server      │   │
│                              │  Port: 9000           │   │
│                              │  - POST /stake        │   │
│                              │  - GET /robot/status  │   │
│                              └────────────┬──────────┘   │
└─────────────────────────────────────────┼──────────────┘
                                           │
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
        ┌───────────────────┐  ┌──────────────────┐  ┌──────────────────┐
        │   Trust Service   │  │ Refinery Service │  │   NATS Message   │
        │   Port: 8083      │  │  Port: 8081      │  │   Port: 4222     │
        │                   │  │                  │  │                  │
        │ Sends RoboStake   │  │ Receives Ore     │  │ (Future: Events) │
        │ via POST /stake   │  │ Mints Ingots     │  │                  │
        └───────────────────┘  └──────────────────┘  └──────────────────┘
```

---

## Configuration

### Environment Variables

**REFINERY_URL**: Base URL for Refinery service
- Default: `http://localhost:8081`
- Example: `export REFINERY_URL=http://refinery.prod:8080`

### Contract Configuration

**Located in**: `src-tauri/src/main.rs`

```rust
Contract {
    id: "contract-001",         // Job ID
    authorized: true,            // Approval status
    torq: 5,                     // Work multiplier (5× normal pay)
    max_token_throughput: 12,    // Tokens per second
    interval_seconds: 5,         // Milestone interval
    total_tokens: 0,             // Running total
    robo_stake_total: 0.0,       // Set by POST /stake
    duration_hours: 0.0,         // Calculated from stake
}
```

### Digger Configuration

```rust
Digger {
    id: "dig-jon-ai-001",       // Robot ID
    power_kw: 0.25,              // Power consumption (250W)
    max_token_throughput: 12,    // Max tokens/second
    current_contract: Some(...), // Active contract
}
```

---

## Future Enhancements

### Planned (TODO #2)
- **Cryptographic Signatures**: Post-quantum (Dilithium) signatures for ore authenticity
- **Proof of Work**: Optional visual proof generation

### Potential Improvements
- **Dynamic Pricing**: Adjust token generation based on network difficulty
- **Multi-Digger Support**: Manage multiple robots simultaneously
- **Retry Logic**: Exponential backoff for Refinery connection failures
- **Persistent Storage**: Save contract state to disk (resume after restart)
- **NATS Integration**: Publish contract events to message bus
- **WebSocket Updates**: Real-time dashboard via WebSocket instead of polling

---

## Troubleshooting

### Common Issues

**"Contract not found for robot"**
- Ensure RoboStake was sent via POST /stake before starting contract
- Verify contract_id matches between stake request and start_contract call

**"Connection refused: tcp connect error"**
- Check Refinery is running: `docker ps | grep refinery`
- Verify Refinery port: `docker logs robotorq-network-refinery-1`
- Test endpoint: `curl http://localhost:8081/health`

**Dashboard not updating**
- Check browser console for JavaScript errors
- Verify event listeners are registered
- Confirm contract is actually running (check terminal logs)

**"Cannot drop a runtime in a context where blocking is not allowed"**
- This was fixed by converting `send_ore_to_refinery()` to async
- If you see this, ensure all HTTP operations use async reqwest, not blocking

### Debug Commands

```bash
# Check Digger logs
cargo tauri dev

# Check Refinery logs
docker logs -f robotorq-network-refinery-1

# Test stake endpoint
curl -X POST http://localhost:9000/stake \
  -H "Content-Type: application/json" \
  -d '{"contract_id":"test-1","amount_rt":10.0}'

# Check contract status
curl http://localhost:9000/robot/status
```

---

## Contributing

When making changes to the Digger:

1. **Run unit tests**: `cargo test`
2. **Test manually**: Follow scenarios in `TESTING_PLAN.md`
3. **Check for compiler warnings**: `cargo clippy`
4. **Update this documentation** if adding new features
5. **Commit with descriptive messages**

### Branch Strategy

- `main`: Stable production code
- `MVP`: Current development branch
- `feature/digger-refactor`: Feature branches for major changes

---

## License

Part of the RoboTorq Network - Project Asimov
