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