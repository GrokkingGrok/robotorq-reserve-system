// This file is the **Digger Dashboard** — the control panel you see on screen

console.log("Digger Dashboard frontend loaded!");

// ────────────────────────────────────────────────────────────────
// TAURI MAGIC — How the screen talks to the robot brain
// ────────────────────────────────────────────────────────────────

// These are like **walkie-talkies** to the Rust code in the back
// We get them from a special box that Tauri gives us
const { invoke } = window.__TAURI__.core;   // Send commands to Rust
const { listen } = window.__TAURI__.event;  // Listen for updates from Rust

// ────────────────────────────────────────────────────────────────
// SCREEN ELEMENTS — The buttons and text boxes you see
// ────────────────────────────────────────────────────────────────

// These are like **handles** to parts of the webpage
const diggerInput = document.getElementById('digger_id');     // Robot name box
const contractInput = document.getElementById('contract_id'); // Job number box
const startBtn = document.getElementById('startBtn');         // Start button
const statusEl = document.getElementById('status');           // Top status line
const outputEl = document.getElementById('output');           // Big text area

// TODO #10: Dashboard elements ✅
const pauseBtn = document.getElementById('pauseBtn');
const resumeBtn = document.getElementById('resumeBtn');
const stopBtn = document.getElementById('stopBtn');
const controlsPanel = document.getElementById('controls');
const economicsPanel = document.getElementById('economics');
const progressPanel = document.getElementById('progress');
const healthPanel = document.getElementById('health');

// Track current contract ID for pause/resume/stop
let currentContractId = null;

// ────────────────────────────────────────────────────────────────
// HELPER FUNCTIONS
// ────────────────────────────────────────────────────────────────

/// Format seconds into "Xh Ym Zs"
function formatTime(seconds) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  return `${hours}h ${minutes}m ${secs}s`;
}

/// Format large numbers with commas
function formatNumber(num) {
  return num.toLocaleString();
}

// ────────────────────────────────────────────────────────────────
// HELPER: Write messages to the screen and console
// ────────────────────────────────────────────────────────────────

/// Print a message both in the browser console AND on the screen
function log(msg) {
  console.log(msg);                        // Show in developer tools
  outputEl.textContent += `\n${msg}`;      // Show in the big text box
}

// Safety check: Make sure Tauri is actually running
if (!window.__TAURI__ || !invoke) {
  console.error("Tauri API not found. Are you running inside Tauri?");
}

// ────────────────────────────────────────────────────────────────
// CONTRACT CONTROL FUNCTIONS (TODO #7)
// ────────────────────────────────────────────────────────────────

async function pauseContract() {
  if (!currentContractId) return;
  
  try {
    await invoke("pause_contract", { contractId: currentContractId });
    log(`⏸️ Contract ${currentContractId} paused`);
    pauseBtn.classList.add('hidden');
    resumeBtn.classList.remove('hidden');
  } catch (err) {
    log(`Error pausing contract: ${err}`);
  }
}

async function resumeContract() {
  if (!currentContractId) return;
  
  try {
    await invoke("resume_contract", { contractId: currentContractId });
    log(`▶️ Contract ${currentContractId} resumed`);
    resumeBtn.classList.add('hidden');
    pauseBtn.classList.remove('hidden');
  } catch (err) {
    log(`Error resuming contract: ${err}`);
  }
}

async function stopContract() {
  if (!currentContractId) return;
  
  if (!confirm('Are you sure you want to stop this contract? This cannot be undone.')) {
    return;
  }
  
  try {
    await invoke("stop_contract", { contractId: currentContractId });
    log(`🛑 Contract ${currentContractId} stopped`);
    stopBtn.disabled = true;
    pauseBtn.disabled = true;
  } catch (err) {
    log(`Error stopping contract: ${err}`);
  }
}

// Connect control buttons
pauseBtn.addEventListener('click', pauseContract);
resumeBtn.addEventListener('click', resumeContract);
stopBtn.addEventListener('click', stopContract);

// ────────────────────────────────────────────────────────────────
// MAIN ACTION: Start a job when you click the button
// ────────────────────────────────────────────────────────────────

async function startContract() {
  // Get what the user typed
  const digger_id = diggerInput.value.trim();     // Robot name
  const contract_id = contractInput.value.trim(); // Job number

  // Tell everyone we're starting
  log(`Starting contract with Digger: ${digger_id}, Contract: ${contract_id}`);
  startBtn.disabled = true;           // Disable button so you can't click twice
  statusEl.textContent = `Starting ${contract_id}...`;
  outputEl.textContent = 'Initializing...';
  
  // Store current contract ID for controls
  currentContractId = contract_id;
  
  // Show dashboard panels
  controlsPanel.classList.remove('hidden');
  economicsPanel.classList.remove('hidden');
  progressPanel.classList.remove('hidden');
  healthPanel.classList.remove('hidden');
  
  // Reset control buttons
  pauseBtn.classList.remove('hidden');
  resumeBtn.classList.add('hidden');
  stopBtn.disabled = false;
  pauseBtn.disabled = false;

  try {
    // Send the command to Rust: "Start this job!"
    const jobId = await invoke("start_contract", {
      diggerId: digger_id,
      contractId: contract_id
    });

    // Success! We got a unique job ID back
    log(`Contract started! Job ID: ${jobId}`);
    statusEl.textContent = `Contract ${jobId} running...`;

    // ────────────────────────────────────────────────────────────────
    // TODO #10: Subscribe to Contract Status Updates ✅ IMPLEMENTED
    // ────────────────────────────────────────────────────────────────
    
    // Listen for contract started event
    const unlistenStarted = await listen("contract_started", (event) => {
      const data = event.payload;
      log(`📝 Contract initialized: ${data.total_milestones} milestones over ${data.duration_hours}h`);
    });
    
    // Listen for comprehensive status updates
    const unlistenStatus = await listen("contract_status_update", (event) => {
      const data = event.payload;
      
      // Economics panel
      document.getElementById("total-tokens").textContent = formatNumber(data.total_tokens);
      document.getElementById("total-joules").textContent = `${formatNumber(data.total_joules)} J`;
      document.getElementById("robostake-received").textContent = `${data.total_robo_stake.toFixed(2)} RT`;
      document.getElementById("robostake-sent").textContent = `${data.robo_stake_sent.toFixed(6)} RT`;
      
      // Progress panel
      const percent = data.percent_complete;
      document.getElementById("progress-bar").style.width = `${percent}%`;
      document.getElementById("milestones").textContent = `${data.current_milestone + 1} / ${data.total_milestones}`;
      document.getElementById("percent-complete").textContent = `${percent.toFixed(1)}%`;
      document.getElementById("time-elapsed").textContent = formatTime(data.time_elapsed_secs);
      document.getElementById("time-remaining").textContent = formatTime(data.time_remaining_secs);
      
      // Health panel
      const healthIndicator = document.getElementById("refinery-health");
      if (data.refinery_healthy) {
        healthIndicator.classList.add("healthy");
        healthIndicator.classList.remove("unhealthy");
        document.getElementById("refinery-status").textContent = "🟢 Online";
      } else {
        healthIndicator.classList.add("unhealthy");
        healthIndicator.classList.remove("healthy");
        document.getElementById("refinery-status").textContent = "🔴 Offline";
      }
      document.getElementById("milestones-confirmed").textContent = data.milestones_confirmed;
      document.getElementById("milestones-failed").textContent = data.milestones_failed;
    });
    
    // Listen for milestone confirmations
    const unlistenConfirmed = await listen("milestone_confirmed", (event) => {
      const data = event.payload;
      log(`✅ Milestone ${data.milestone_index} confirmed (${data.robo_stake.toFixed(6)} RT)`);
    });
    
    // Listen for milestone failures
    const unlistenFailed = await listen("milestone_failed", (event) => {
      const data = event.payload;
      log(`⚠️  Milestone ${data.milestone_index} failed: ${data.error}`);
    });
    
    // Listen for contract paused
    const unlistenPaused = await listen("contract_paused", (event) => {
      log(`⏸️  Contract paused`);
      statusEl.textContent = `Contract ${contract_id} paused`;
    });
    
    // Listen for contract resumed
    const unlistenResumed = await listen("contract_resumed", (event) => {
      log(`▶️  Contract resumed`);
      statusEl.textContent = `Contract ${contract_id} running...`;
    });

    // Listen for **live updates** every few seconds (legacy)
    const unlistenUpdate = await listen(`ore_update_${jobId}`, (event) => {
      // Kept for backward compatibility, but main UI uses contract_status_update
    });

    // Listen for **job finished** message
    const unlistenComplete = await listen("contract_completed", (event) => {
      log(`✅ Contract ${contract_id} completed!`);
      statusEl.textContent = `Contract ${contract_id} complete!`;
      statusEl.className = 'status complete';
      
      // Hide controls, keep panels visible to show final stats
      controlsPanel.classList.add('hidden');
      
      // Re-enable start button and stop listening
      startBtn.disabled = false;
      currentContractId = null;
      
      unlistenStarted();
      unlistenStatus();
      unlistenConfirmed();
      unlistenFailed();
      unlistenPaused();
      unlistenResumed();
      unlistenUpdate();
      unlistenComplete();
    });
    
    // Listen for contract stopped by user
    const unlistenStopped = await listen("contract_stopped", (event) => {
      log(`🛑 Contract ${contract_id} stopped`);
      statusEl.textContent = `Contract ${contract_id} stopped`;
      statusEl.className = 'status error';
      
      // Hide controls
      controlsPanel.classList.add('hidden');
      
      // Re-enable start button
      startBtn.disabled = false;
      currentContractId = null;
      
      unlistenStarted();
      unlistenStatus();
      unlistenConfirmed();
      unlistenFailed();
      unlistenPaused();
      unlistenResumed();
      unlistenUpdate();
      unlistenComplete();
      unlistenStopped();
    });

  } catch (err) {
    // Something went wrong
    log(`Error: ${err}`);
    statusEl.textContent = `Failed: ${err}`;
    statusEl.className = 'status error';
    startBtn.disabled = false;
    currentContractId = null;
    
    // Hide panels
    controlsPanel.classList.add('hidden');
    economicsPanel.classList.add('hidden');
    progressPanel.classList.add('hidden');
    healthPanel.classList.add('hidden');
  }
}

// ────────────────────────────────────────────────────────────────
// CONNECT THE BUTTON
// ────────────────────────────────────────────────────────────────

// When someone clicks the Start button, run the function above
startBtn.addEventListener('click', startContract);