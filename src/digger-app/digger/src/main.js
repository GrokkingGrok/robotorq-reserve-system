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

  try {
    // Send the command to Rust: "Start this job!"
    const jobId = await invoke("start_contract", {
      diggerId: digger_id,
      contractId: contract_id
    });

    // Success! We got a unique job ID back
    log(`Contract started! Job ID: ${jobId}`);
    statusEl.textContent = `Contract ${jobId} running...`;

    // Listen for **live updates** every few seconds
    const unlistenUpdate = await listen(`ore_update_${jobId}`, (event) => {
      log(`Live Update: ${JSON.stringify(event.payload, null, 2)}`);
    });

    // ────────────────────────────────────────────────────────────────
    // TODO #10: Subscribe to Contract Status Updates
    // ────────────────────────────────────────────────────────────────
    // CURRENT: Only listens to ore_update (raw ore data)
    // NEEDED: Listen to contract_status_update (rich economics/progress)
    //
    // IMPLEMENTATION:
    // 1. Add listener for "contract_status_update" event (from TODO #9)
    // 2. Update dashboard elements with received data:
    //
    //    const unlistenStatus = await listen("contract_status_update", (event) => {
    //      const data = event.payload;
    //      
    //      // Economics panel
    //      document.getElementById("total-jouletorq").textContent = 
    //        `${data.total_joules} J + ${data.total_tokens} tokens`;
    //      document.getElementById("robostake-received").textContent = 
    //        `${data.total_robo_stake.toFixed(2)} RT`;
    //      document.getElementById("robostake-sent").textContent = 
    //        `${data.robo_stake_sent.toFixed(2)} RT`;
    //      
    //      // Progress panel
    //      const percent = (data.current_milestone / data.total_milestones) * 100;
    //      document.getElementById("progress-bar").style.width = `${percent}%`;
    //      document.getElementById("milestones").textContent = 
    //        `${data.current_milestone} / ${data.total_milestones}`;
    //      document.getElementById("time-elapsed").textContent = 
    //        formatTime(data.time_elapsed_secs);
    //      document.getElementById("time-remaining").textContent = 
    //        formatTime(data.time_remaining_secs);
    //      
    //      // Health panel
    //      const healthIndicator = document.getElementById("refinery-health");
    //      if (data.refinery_healthy) {
    //        healthIndicator.classList.add("healthy");
    //        healthIndicator.classList.remove("unhealthy");
    //        document.getElementById("refinery-status").textContent = "Online";
    //      } else {
    //        healthIndicator.classList.add("unhealthy");
    //        healthIndicator.classList.remove("healthy");
    //        document.getElementById("refinery-status").textContent = "Offline";
    //      }
    //      document.getElementById("milestones-confirmed").textContent = 
    //        data.milestones_confirmed;
    //      document.getElementById("milestones-failed").textContent = 
    //        data.milestones_failed;
    //    });
    //
    // 3. Add formatTime() helper function:
    //    function formatTime(seconds) {
    //      const hours = Math.floor(seconds / 3600);
    //      const minutes = Math.floor((seconds % 3600) / 60);
    //      const secs = seconds % 60;
    //      return `${hours}h ${minutes}m ${secs}s`;
    //    }
    //
    // 4. Clean up listener on completion:
    //    unlistenStatus(); // in ore_complete handler
    // ────────────────────────────────────────────────────────────────

    // Listen for **job finished** message
    const unlistenComplete = await listen(`ore_complete_${jobId}`, (event) => {
      log(`Contract ${jobId} complete!`);
      statusEl.textContent = `Contract ${jobId} complete!`;
      log(`Final Ore: ${JSON.stringify(event.payload, null, 2)}`);
      
      // Re-enable the button and stop listening
      startBtn.disabled = false;
      unlistenUpdate();
      unlistenComplete();
    });

  } catch (err) {
    // Something went wrong
    log(`Error: ${err}`);
    statusEl.textContent = `Failed: ${err}`;
    startBtn.disabled = false;
  }
}

// ────────────────────────────────────────────────────────────────
// CONNECT THE BUTTON
// ────────────────────────────────────────────────────────────────

// When someone clicks the Start button, run the function above
startBtn.addEventListener('click', startContract);