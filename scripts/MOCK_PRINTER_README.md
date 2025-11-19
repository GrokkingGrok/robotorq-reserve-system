# Mock Printer - Milestone Reporting

## Overview

The Mock Printer simulates a physical 3D printer that creates RoboTorq tokens. It communicates with the Digger service via HTTP to:

1. Create contracts
2. Pay stake
3. Execute contracts (generate JTUs)
4. **Report milestones with photos** (NEW!)

## Architecture

```
┌─────────────────┐           HTTP              ┌─────────────────┐
│  Mock Printer   │ ────────────────────────────▶│     Digger      │
│  (Python)       │                              │   (Rust HTTP)   │
│                 │  POST /contracts/create      │                 │
│                 │  POST /contracts/stake       │  - Manages      │
│                 │  POST /contracts/execute     │    contracts    │
│                 │  POST /contracts/:id/        │  - Generates    │
│                 │       milestone              │    JTUs         │
│                 │                              │  - Tracks       │
│                 │  ◀─── milestone response    │    milestones   │
└─────────────────┘                              └─────────────────┘
```

## New Feature: Milestone Reporting

### Digger Changes

Added new HTTP endpoint:

**`POST /contracts/{contract_id}/milestone`**

Request body:
```json
{
  "milestone_number": 1,
  "photo_base64": "iVBORw0KGgo...",  // Optional
  "notes": "Print bed at 50%"        // Optional
}
```

Response:
```json
{
  "contract_id": "contract-001",
  "milestone_number": 1,
  "milestones_completed": 1,
  "milestones_total": 5,
  "progress": 0.2,
  "is_complete": false,
  "message": "Milestone 1 of 5 complete (20.0% done)"
}
```

### Mock Printer Changes

The printer now:
1. Calls `/contracts/execute` to start (Digger generates all JTUs immediately)
2. Simulates printing with sleep intervals
3. Reports each milestone to Digger with photo and progress notes
4. Digger tracks milestone completion and updates contract state

## Usage

### 1. Start Digger

```powershell
cd src/digger
cargo run
```

Digger runs on `http://localhost:9000`

### 2. Run Mock Printer

```powershell
# Basic usage (30 second print, 5 milestones)
python scripts/mock_printer.py

# Custom configuration
python scripts/mock_printer.py \
    --contract-id my-print-001 \
    --duration 20 \
    --milestones 4 \
    --torq 100.0 \
    --robo-stake 5.0 \
    --power 2000.0

# Point to remote Digger
python scripts/mock_printer.py --digger-url http://192.168.1.100:9000
```

### 3. Run Test Script

```bash
# Quick test (20 seconds, 4 milestones)
python scripts/test_mock_printer.py
```

## Example Output

```
============================================================
  Mock Printer - Simulating Physical RT Print Job
============================================================

ℹ️  Creating contract mock-contract-1700000000...
✅ Contract created: mock-contract-1700000000
ℹ️    Ore target: 500.0 RT

ℹ️  Paying stake for contract mock-contract-1700000000...
✅ Stake paid: StakeApproved

============================================================
  🖨️  PRINTER STARTED
============================================================
ℹ️  Contract: mock-contract-1700000000
ℹ️  Duration: 20 seconds
ℹ️  Milestones: 4
ℹ️  Simulating physical RT printing...

ℹ️  Starting contract execution on Digger...
✅ Contract execution started!
ℹ️    JTUs generated: 40000
ℹ️    Ore generated: 500.00 RT
ℹ️    Ore target: 500.00 RT
ℹ️    Target reached: True

ℹ️  📸 Milestone 1/4 (25.0% complete, 5.0s elapsed)
✅   Milestone reported: 1/4
ℹ️    ✓ Milestone 1 acknowledged by Digger

ℹ️  📸 Milestone 2/4 (50.0% complete, 10.0s elapsed)
✅   Milestone reported: 2/4
ℹ️    ✓ Milestone 2 acknowledged by Digger

ℹ️  📸 Milestone 3/4 (75.0% complete, 15.0s elapsed)
✅   Milestone reported: 3/4
ℹ️    ✓ Milestone 3 acknowledged by Digger

ℹ️  📸 Milestone 4/4 (100.0% complete, 20.0s elapsed)
✅   Milestone reported: 4/4
ℹ️    ✓ Milestone 4 acknowledged by Digger

============================================================
  ✅ PRINT JOB COMPLETE
============================================================
✅ Contract mock-contract-1700000000 complete!
ℹ️    JTUs generated: 40000
ℹ️    Ore generated: 500.00 RT
ℹ️    Ore target: 500.00 RT
ℹ️    Target reached: True
ℹ️    Progress: 100.0%

============================================================
  🎉 Mock Printer Cycle Complete!
============================================================
```

## Implementation Details

### Contract Lifecycle

1. **Create Contract**: `POST /contracts/create`
   - Defines torq (value), robo_stake (cost), milestones, power
   - Status: `PendingStake`

2. **Pay Stake**: `POST /contracts/stake`
   - Approves contract for execution
   - Status: `StakeApproved`

3. **Execute Contract**: `POST /contracts/execute`
   - Digger generates all JTUs immediately
   - Calculates ore based on torq × robo_stake
   - Contract still in `StakeApproved` status

4. **Report Milestones**: `POST /contracts/{id}/milestone` (x4)
   - Printer reports progress with photos
   - Each report increments milestone counter
   - Final milestone transitions to `ExecutionComplete`

5. **Query Status**: `GET /contracts/{id}`
   - Check progress, JTU count, ore generated

### Data Flow

```
Printer                          Digger
   │                               │
   │─── POST /contracts/create ───▶│ Create contract
   │◀──── ore_target: 500 RT ──────│
   │                               │
   │─── POST /contracts/stake ────▶│ Approve contract
   │◀──── StakeApproved ───────────│
   │                               │
   │─── POST /contracts/execute ──▶│ Generate 40,000 JTUs
   │◀──── JTUs: 40000 ─────────────│ Ore: 500 RT
   │                               │
   │    (sleep 5 seconds)          │
   │─── POST .../milestone #1 ────▶│ Increment milestone
   │◀──── progress: 25% ───────────│
   │                               │
   │    (sleep 5 seconds)          │
   │─── POST .../milestone #2 ────▶│ Increment milestone
   │◀──── progress: 50% ───────────│
   │                               │
   │    (sleep 5 seconds)          │
   │─── POST .../milestone #3 ────▶│ Increment milestone
   │◀──── progress: 75% ───────────│
   │                               │
   │    (sleep 5 seconds)          │
   │─── POST .../milestone #4 ────▶│ ExecutionComplete
   │◀──── progress: 100% ──────────│
   │                               │
   │─── GET /contracts/{id} ───────▶│ Final status
   │◀──── Complete! ───────────────│
```

## Future Enhancements

- [ ] Add photo validation (check base64 format)
- [ ] Store milestone photos in Digger database
- [ ] Add milestone timing validation (detect too-fast progress)
- [ ] Support pausing/resuming contracts
- [ ] Add real-time progress streaming (WebSocket)
- [ ] Multi-printer coordination (one contract, many printers)

## Why This Approach?

**Simplicity**: Minimal changes to Digger, pure HTTP API  
**Testability**: Easy to simulate printer behavior without hardware  
**Realistic**: Mimics actual printer lifecycle (create → stake → execute → milestones)  
**Extensible**: Foundation for real printer integration later

---

**Status**: ✅ Implemented and tested  
**Date**: November 19, 2025  
**Branch**: `feature/mockprinter`
