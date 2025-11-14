# TODO #12.1: Human Testing Plan for Digger App

## Overview
This testing plan covers manual verification of all features implemented in TODOs #3-10. Run through these scenarios to ensure the complete Digger → Refinery integration works end-to-end.

---

## Prerequisites

### 1. **Start Required Services**
```powershell
# Start NATS message broker
cd C:\Users\Jon\Documents\Project-Asimov\robotorq-network
docker-compose up -d nats

# Start Trust service (for RoboStake staking)
cd C:\Users\Jon\Documents\Project-Asimov\robotorq-network\src\trust
$env:NATS_URL="nats://127.0.0.1:4222"
go run ./cmd/trust

# Start Refinery service (for ore processing)
cd C:\Users\Jon\Documents\Project-Asimov\robotorq-network\src\refinery
cargo run --release
```

### 2. **Launch Digger Dashboard**
```powershell
cd C:\Users\Jon\Documents\Project-Asimov\robotorq-network\src\digger-app\digger
cargo tauri dev
```

---

## Test Scenarios

### **Test #1: Basic Contract Start**
**Purpose:** Verify contract initialization and basic execution

**Steps:**
1. Open Digger Dashboard (should launch automatically with `cargo tauri dev`)
2. Verify default values:
   - Digger ID: `dig-jon-ai-001`
   - Contract ID: `contract-001`
3. Click **"Start Contract"** button
4. **Expected Results:**
   - Button becomes disabled
   - Status changes to "Contract contract-001 running..."
   - Dashboard panels appear:
     - Economics panel (💰 Economics)
     - Progress panel (⚙️ Progress) 
     - Health panel (🔍 Status)
   - Control buttons appear (⏸️ Pause, 🛑 Stop)
   - Output log shows: "Contract started! Job ID: Robot_dig-jon-ai-001_started_contract_contract-001"

**Validation:**
- [ ] Contract starts successfully
- [ ] All dashboard panels visible
- [ ] Initial values show:
  - Tokens: 0
  - Joules: 0 J
  - RoboStake Received: 0.00 RT (will update when Trust sends stake)
  - Milestones: 1 / [total milestones]
  - Progress bar starts at 0%

---

### **Test #2: RoboStake Integration (TODO #3)**
**Purpose:** Verify Trust service sends RoboStake and duration calculation is correct

**Prerequisites:** Trust service must be running

**Steps:**
1. Monitor Trust service logs in its terminal
2. Wait for Trust to send POST request to Digger's HTTP API (port 9000)
3. Check Digger HTTP API logs for `/stake` endpoint hit
4. Observe Economics panel updates

**Expected Results:**
- Trust sends: `POST http://localhost:9000/stake` with:
  ```json
  {
    "digger_id": "dig-jon-ai-001",
    "contract_id": "contract-001",
    "robo_stake_amount": 3000.0
  }
  ```
- Digger calculates `duration_hours` based on robo_stake_amount
- Economics panel updates:
  - RoboStake Received: `3000.00 RT`
  - Total milestones recalculated based on duration

**Validation:**
- [ ] RoboStake Received shows correct amount (3000.00 RT)
- [ ] Duration calculated correctly (check output logs)
- [ ] Total milestones = (duration_hours × 3600) / interval_seconds

---

### **Test #3: Real-Time Dashboard Updates (TODO #9)**
**Purpose:** Verify contract_status_update events update UI in real-time

**Steps:**
1. Let contract run for 30-60 seconds
2. Observe all dashboard panels updating every 5 seconds (interval_seconds)

**Expected Results (every 5 seconds):**

**Economics Panel:**
- Tokens Generated increases (e.g., 60, 120, 180...)
- Energy Consumed increases (e.g., 1250 J, 2500 J...)
- RoboStake Sent gradually approaches RoboStake Received

**Progress Panel:**
- Progress bar advances smoothly
- Milestones: "X / Y" increments (e.g., "5 / 72000")
- Percent Complete increases (e.g., 0.1%, 0.2%...)
- Time Elapsed counts up (e.g., "0h 0m 15s", "0h 0m 20s")
- Time Remaining counts down

**Health Panel:**
- Refinery Connection: 🟢 Online (if Refinery running)
- Milestones Confirmed increases
- Milestones Failed stays at 0 (if healthy)

**Validation:**
- [ ] All metrics update every 5 seconds
- [ ] Progress bar animates smoothly
- [ ] Time elapsed/remaining calculations accurate
- [ ] No UI freezing or lag

---

### **Test #4: Refinery Integration (TODO #4, #6)**
**Purpose:** Verify ore is sent to Refinery with RoboStake amounts

**Prerequisites:** Refinery service must be running

**Steps:**
1. Monitor Refinery logs in its terminal
2. Watch for `POST /receive-ore` requests every 5 seconds
3. Check milestone confirmed events in Digger output log

**Expected Results:**
- Digger sends ore to Refinery every 5 seconds:
  ```json
  {
    "digger_id": "dig-jon-ai-001",
    "contract_id": "contract-001",
    "tokens_generated": 60,
    "joules": 1250,
    "milestone_index": 0,
    "timestamp": 1731623400,
    "proof_of_work": "cGhvdG8tZGlnLWpvbi1haS0wMDEtMA==",
    "robo_stake_amount": 0.208333,
    "signature": null
  }
  ```
- Refinery responds with HTTP 200
- Digger logs: `✅ Refinery accepted milestone X`
- Output shows: `✅ Milestone X confirmed (0.208333 RT)`

**Validation:**
- [ ] Ore sent every interval_seconds (5s)
- [ ] robo_stake_amount correct (robo_stake_total / total_milestones)
- [ ] Refinery accepts all milestones (200 OK)
- [ ] Milestones Confirmed count matches milestone_index

---

### **Test #5: Milestone Status Tracking (TODO #8)**
**Purpose:** Verify milestone success/failure tracking

**Steps:**
1. **With Refinery Running:**
   - Let contract run for 30 seconds
   - Check Health panel → Milestones Confirmed
   - Verify Refinery Connection shows 🟢 Online
   
2. **With Refinery Stopped:**
   - Stop Refinery service (Ctrl+C in Refinery terminal)
   - Wait for next milestone (5 seconds)
   - Check Digger output log for failure message

**Expected Results:**

**Refinery Running:**
- Milestones Confirmed increments every 5 seconds
- Milestones Failed stays at 0
- Refinery status: 🟢 Online
- Output: `✅ Milestone X confirmed (Y RT)`

**Refinery Stopped:**
- Milestones Failed increments
- Refinery status changes to 🔴 Offline
- Output: `⚠️ Milestone X failed: NetworkError: ...`
- Contract continues running (ore stored locally)

**Validation:**
- [ ] Confirmed count accurate when Refinery healthy
- [ ] Failed count increments when Refinery down
- [ ] Health indicator changes color (green → red)
- [ ] Contract doesn't crash on Refinery errors

---

### **Test #6: Pause/Resume Controls (TODO #7)**
**Purpose:** Verify pause/resume functionality

**Steps:**
1. Let contract run for 15 seconds
2. Click **⏸️ Pause** button
3. Wait 10 seconds
4. Click **▶️ Resume** button
5. Let contract run for 15 more seconds

**Expected Results:**

**After Pause:**
- ⏸️ Pause button becomes hidden
- ▶️ Resume button appears
- Status changes to "Contract contract-001 paused"
- Output log: `⏸️ Contract paused`
- Dashboard metrics **freeze** (no updates)
- Progress bar stops advancing
- Time Elapsed stops incrementing

**After Resume:**
- ▶️ Resume button becomes hidden
- ⏸️ Pause button appears
- Status returns to "Contract contract-001 running..."
- Output log: `▶️ Contract resumed`
- Dashboard updates resume immediately
- Time Remaining picks up where it left off

**Validation:**
- [ ] Pause stops all processing
- [ ] Dashboard freezes during pause
- [ ] Resume continues from exact state
- [ ] No milestones sent while paused
- [ ] No duplicate processing after resume

---

### **Test #7: Stop Contract (TODO #7)**
**Purpose:** Verify stop functionality and cleanup

**Steps:**
1. Start a new contract
2. Let it run for 30 seconds
3. Click **🛑 Stop** button
4. Confirm the dialog: "Are you sure..."
5. Observe shutdown behavior

**Expected Results:**
- Confirmation dialog appears
- After clicking OK:
  - Output log: `🛑 Contract contract-001 stopped`
  - Status changes to "Contract contract-001 stopped"
  - Status line turns red
  - Control panel (Pause/Stop buttons) disappears
  - Dashboard panels remain visible with final stats
  - "Start Contract" button re-enables
  - No more milestone processing occurs

**Validation:**
- [ ] Confirmation required (prevents accidental stops)
- [ ] Contract stops immediately
- [ ] All event listeners cleaned up
- [ ] No memory leaks (can start new contract)
- [ ] Final stats preserved on screen

---

### **Test #8: Timer-Based Completion (TODO #7)**
**Purpose:** Verify contract completes naturally after duration_hours

**Prerequisites:** This requires a short duration contract. Modify initial contract duration or run a test with very small robo_stake_total.

**Steps:**
1. Option A: Wait for full 20-hour duration (not practical)
2. Option B: Test with modified contract:
   - In `main.rs`, change initial contract `duration_hours` to `0.016` (≈1 minute)
   - Or stake a small amount from Trust (e.g., 5 RT for ~6 second duration)
3. Let contract run to completion

**Expected Results:**
- Progress bar reaches 100%
- Percent Complete shows "100.0%"
- Time Remaining counts down to "0h 0m 0s"
- Output log: `✅ Contract contract-001 completed!`
- Status changes to "Contract contract-001 complete!"
- Status line turns cyan
- Control panel disappears
- Dashboard shows final statistics:
  - Total tokens generated
  - Total energy consumed
  - RoboStake sent equals RoboStake received
  - All milestones confirmed

**Validation:**
- [ ] Contract completes at exact duration
- [ ] Final RoboStake sent ≈ RoboStake received (within rounding)
- [ ] All milestones processed
- [ ] Completion event emitted
- [ ] Start button re-enables for new contract

---

### **Test #9: Multiple Contracts (Isolation)**
**Purpose:** Verify contracts don't interfere with each other

**Steps:**
1. Complete or stop first contract
2. Change Contract ID to `contract-002`
3. Start new contract
4. Verify new contract starts fresh

**Expected Results:**
- New contract initializes with clean state
- Dashboard resets to 0 values
- New contract ID tracked separately
- Old contract data not visible
- Milestone tracking isolated (contract-002 has separate statuses)

**Validation:**
- [ ] Dashboard resets completely
- [ ] No data leakage from previous contract
- [ ] MilestoneTracker correctly isolates contracts
- [ ] ContractStateManager tracks both contracts separately

---

### **Test #10: Error Handling**
**Purpose:** Verify graceful error handling

**Test Cases:**

**A. Invalid Digger ID:**
1. Change Digger ID to `nonexistent-digger`
2. Click Start Contract
3. Expected: Error message "Robot nonexistent-digger not found"
4. UI remains functional

**B. Empty Contract ID:**
1. Clear Contract ID field
2. Click Start Contract
3. Expected: Validation error or meaningful message

**C. Network Timeout:**
1. Set `REFINERY_URL` to invalid address
2. Start contract
3. Expected: Milestones fail gracefully, contract continues

**D. Malformed Stake Request:**
1. Send invalid JSON to `POST /stake`
2. Expected: Error response, doesn't crash

**Validation:**
- [ ] No crashes on invalid input
- [ ] Error messages are clear and actionable
- [ ] UI recovers from errors
- [ ] Logs show detailed error information

---

## Performance Checks

### **Memory Leak Test**
1. Start and complete 5 contracts in sequence
2. Monitor Task Manager → Digger memory usage
3. Expected: Memory usage stable, no significant growth

### **Long-Running Stability**
1. Run contract for 30+ minutes
2. Expected: No slowdowns, crashes, or UI freezing
3. All metrics continue updating smoothly

### **High Frequency Updates**
1. Modify `interval_seconds` to 1 second
2. Expected: Dashboard handles rapid updates without lag

---

## Edge Cases

### **Edge Case #1: Zero RoboStake**
- Contract starts without Trust staking
- Expected: RoboStake Received shows 0.00 RT
- robo_per_milestone calculated as 0.0
- Contract still runs, generates tokens/joules

### **Edge Case #2: Pause During Milestone Send**
- Pause contract mid-milestone
- Expected: Current milestone completes, then pauses
- No corrupted state

### **Edge Case #3: Stop During Refinery Request**
- Stop contract while waiting for Refinery response
- Expected: Cleanup happens gracefully
- No orphaned HTTP requests

---

## Success Criteria

### ✅ **Must Pass:**
- [ ] All test scenarios #1-9 complete successfully
- [ ] No crashes or unhandled errors
- [ ] Dashboard updates in real-time
- [ ] Pause/resume/stop work correctly
- [ ] Economics calculations accurate
- [ ] Refinery integration functional
- [ ] Timer-based completion works

### ✅ **Nice to Have:**
- [ ] Smooth animations and transitions
- [ ] No visible lag in UI updates
- [ ] Error messages are user-friendly
- [ ] Performance stable over time

---

## Bug Reporting Template

If you find issues during testing, document them as:

```markdown
**Bug:** [Brief description]
**Test:** [Which test scenario]
**Steps to Reproduce:**
1. ...
2. ...
3. ...

**Expected:** [What should happen]
**Actual:** [What actually happened]
**Logs:** [Paste relevant console/terminal output]
**Screenshot:** [If applicable]
**Severity:** Critical / High / Medium / Low
```

---

## Post-Testing Checklist

After completing all tests:
- [ ] Document any bugs found
- [ ] Verify all 13 unit tests still pass (`cargo test`)
- [ ] Check for any new compiler warnings
- [ ] Review logs for suspicious errors
- [ ] Confirm all services shut down cleanly
- [ ] Ready for TODO #11 (documentation)

---

## Notes

**Timing Adjustments for Testing:**
- Default 20-hour contract is impractical for testing
- Consider modifying for shorter durations during testing:
  - Change `initial_contract.duration_hours` in `main.rs` to 0.016 (1 minute)
  - Or adjust Trust to stake smaller amounts (5 RT ≈ 6 seconds)

**Refinery Mock Mode:**
- If Refinery isn't available, errors are expected but Digger should handle gracefully
- Milestones will fail but contract continues
- Useful for testing error handling paths

**Logs to Monitor:**
- Digger output (in `cargo tauri dev` terminal)
- Trust service logs (if staking)
- Refinery service logs (if processing ore)
- Browser DevTools console (for frontend errors)
