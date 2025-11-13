console.log("✅ Digger Dashboard frontend loaded!");

// --- Access Tauri APIs via global object (no imports needed)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// --- DOM references
const diggerInput = document.getElementById('digger_id');
const contractInput = document.getElementById('contract_id');
const startBtn = document.getElementById('startBtn');
const statusEl = document.getElementById('status');
const outputEl = document.getElementById('output');

// --- Logging utility
function log(msg) {
  console.log(msg);
  outputEl.textContent += `\n${msg}`;
}

if (!window.__TAURI__ || !invoke) {
  console.error("❌ Tauri API not found. Are you running inside Tauri?");
}

// --- Start Contract logic
async function startContract() {
  const digger_id = diggerInput.value.trim();
  const contract_id = contractInput.value.trim();

  log(`🚀 Starting contract with Digger: ${digger_id}, Contract: ${contract_id}`);
  startBtn.disabled = true;
  statusEl.textContent = `Starting ${contract_id}...`;
  outputEl.textContent = 'Initializing...';

  try {
    // Call Rust backend
    const jobId = await invoke("start_contract", {
      diggerId: digger_id,
      contractId: contract_id
    });

    log(`✅ Contract started! Job ID: ${jobId}`);
    statusEl.textContent = `Contract ${jobId} running...`;

    // Listen for per-second updates
    const unlistenUpdate = await listen(`ore_update_${jobId}`, (event) => {
      log(`📡 Live Update: ${JSON.stringify(event.payload, null, 2)}`);
    });

    // Listen for completion
    const unlistenComplete = await listen(`ore_complete_${jobId}`, (event) => {
      log(`🎉 Contract ${jobId} complete!`);
      statusEl.textContent = `Contract ${jobId} complete!`;
      log(`Final Ore: ${JSON.stringify(event.payload, null, 2)}`);
      startBtn.disabled = false;

      unlistenUpdate();
      unlistenComplete();
    });

  } catch (err) {
    log(`❌ Error: ${err}`);
    statusEl.textContent = `Failed: ${err}`;
    startBtn.disabled = false;
  }
}

// --- Hook up button
startBtn.addEventListener('click', startContract);
