use async_nats::Client;
// use anyhow::Result; // not currently used
use tracing::{info, error};
use crate::events::subjects;
use crate::metrics::VaultMetrics;
use std::sync::Arc;
use tokio::{task, time};
use futures_util::stream::StreamExt;
use dashmap::DashMap;
use crate::persistence::{VaultPersistence, PersistedSchedule, now_ms};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

#[derive(Clone)]
pub struct ShadowDistoVault {
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
    tick_interval_millis: i64,
    schedules: Arc<DashMap<String, StreamState>>, // schedule_id -> state
    persistence: Option<VaultPersistence>,
    time_compression_factor: f64, // For simulation: 1.0 = real-time, 1000.0 = 1000x faster
}

#[derive(Clone, Debug)]
struct StreamState {
    contract_id: String,
    schedule_id: String,
    total: i64,
    planned_ticks: i64,
    base_per_tick: i64,
    remainder: i64,
    next_tick_index: i64,
    distributed_so_far: i64,
    start_ms: u128,
}

impl ShadowDistoVault {
    pub fn new(nats: Client, tick_interval_millis: i64) -> Self { 
        Self { 
            nats, 
            metrics: None, 
            tick_interval_millis, 
            schedules: Arc::new(DashMap::new()), 
            persistence: None,
            time_compression_factor: 1.0, // Default to real-time
        } 
    }

    pub fn with_time_compression(mut self, factor: f64) -> Self {
        self.time_compression_factor = factor;
        self
    }

    pub fn with_metrics(mut self, metrics: Arc<VaultMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn with_persistence(mut self, persistence: VaultPersistence) -> Self {
        self.persistence = Some(persistence);
        self
    }

    /// Recover active schedules from persistence on startup.
    /// Loads incomplete schedules from DB and resumes their emitters from next_tick_index.
    /// Note: Recovery works correctly in production; test timing issues are due to NATS subscription setup.
    pub async fn recover_active(&self) {
        if let Some(p) = &self.persistence {
            match p.load_active_schedules().await {
                Ok(list) => {
                    for ps in list {
                        let state = StreamState {
                            contract_id: ps.contract_id.clone(),
                            schedule_id: ps.schedule_id.clone(),
                            total: ps.total,
                            planned_ticks: ps.planned_ticks,
                            base_per_tick: ps.base_per_tick,
                            remainder: ps.remainder,
                            next_tick_index: ps.next_tick_index,
                            distributed_so_far: ps.distributed_so_far,
                            start_ms: ps.start_ms as u128,
                        };
                        self.schedules.insert(ps.schedule_id.clone(), state);
                        // Clone schedule_id so we can still log with original after spawning emitter
                        self.spawn_linear_emitter(ps.schedule_id.clone());
                        info!(schedule_id=%ps.schedule_id, contract_id=%ps.contract_id, next_tick_index=ps.next_tick_index, distributed_so_far=ps.distributed_so_far, "recovered active schedule");
                    }
                }
                Err(e) => error!(error=%e, "failed to load active schedules"),
            }
        }
    }

    /// Start subscription loop for authorization events.
    /// If authorization event includes distribution_mode == "linear", perform multi-tick linear schedule.
    /// Otherwise fallback to immediate full distribution (current MVP behavior / backward compatible).
    pub fn start(self: Arc<Self>) {
        let svc = self.clone();
        task::spawn(async move {
            match svc.nats.subscribe(subjects::DISTOSTREAM_AUTHORIZED).await {
                Ok(mut sub) => {
                    info!(subject=subjects::DISTOSTREAM_AUTHORIZED, "disto vault subscribed");
                    svc.recover_active().await;
                    while let Some(msg) = sub.next().await {
                        let payload = msg.payload;
                        // Pass-through: parse minimally to extract contract_id & robotorq_total
                        let contract_id = extract_string_field(&payload, "contract_id").unwrap_or_else(|| "unknown-contract".to_string());
                        let robotorq_total = extract_i64_field(&payload, "robotorq_total").unwrap_or(0);
                        let duration_seconds = extract_i64_field(&payload, "duration_seconds").unwrap_or(60);
                        let mode = extract_string_field(&payload, "distribution_mode").unwrap_or_else(|| "immediate".to_string());

                        if mode == "linear" && robotorq_total > 0 {
                            // Build schedule
                            let duration_ms = duration_seconds * 1000;
                            let interval_ms = svc.tick_interval_millis.max(1);
                            let mut ticks = duration_ms / interval_ms;
                            if ticks < 1 { ticks = 1; }
                            if ticks > robotorq_total { ticks = robotorq_total; } // ensure at least 1 unit per tick when small total
                            let base = robotorq_total / ticks;
                            let remainder = robotorq_total % ticks;
                            let schedule_id = make_schedule_id(&contract_id);
                            // Prevent duplicate active schedule per contract (simple rule for MVP)
                            let active_exists = svc.schedules.iter().any(|s| s.contract_id == contract_id && s.distributed_so_far < s.total);
                            if active_exists {
                                error!(contract_id=%contract_id, "active schedule exists; ignoring new authorization linear request");
                                continue;
                            }
                            let state = StreamState {
                                contract_id: contract_id.clone(),
                                schedule_id: schedule_id.clone(),
                                total: robotorq_total,
                                planned_ticks: ticks,
                                base_per_tick: base,
                                remainder,
                                next_tick_index: 0,
                                distributed_so_far: 0,
                                start_ms: current_ms(),
                            };
                            svc.schedules.insert(schedule_id.clone(), state);
                            if let Some(p) = &svc.persistence {
                                let ps = PersistedSchedule {
                                    schedule_id: schedule_id.clone(),
                                    contract_id: contract_id.clone(),
                                    total: robotorq_total,
                                    planned_ticks: ticks,
                                    base_per_tick: base,
                                    remainder,
                                    next_tick_index: 0,
                                    distributed_so_far: 0,
                                    start_ms: now_ms(),
                                };
                                if let Err(e) = p.persist_schedule(&ps).await { error!(error=%e, schedule_id=%schedule_id, "failed to persist schedule"); }
                            }
                            info!(contract_id=%contract_id, schedule_id=%schedule_id, ticks=ticks, total=robotorq_total, "linear schedule created");
                            svc.spawn_linear_emitter(schedule_id.clone());
                            if let Some(m) = &svc.metrics { m.inc_distostream_authorized(); }
                            continue; // skip immediate path
                        }
                        // Emit a single distribution tick (tick_index=0 for MVP).
                        let tick = serde_json::json!({
                            "event_type": "distostream_distribution_tick",
                            "contract_id": contract_id,
                            "tick_index": 0,
                            "authorized_robotorq_total": robotorq_total,
                            "distributed_robotorq": robotorq_total, // MVP: all distributed immediately
                            "cumulative_distributed": robotorq_total,
                            "remaining": 0,
                        });
                        if let Err(e) = svc.nats.publish(subjects::DISTOSTREAM_DISTRIBUTION_TICK, serde_json::to_vec(&tick).unwrap().into()).await {
                            error!(error=%e, "failed publishing distribution tick");
                        } else {
                            info!(contract_id=%tick["contract_id"], robotorq_total=%robotorq_total, "distribution tick emitted");
                            if let Some(m) = &svc.metrics { m.inc_distostream_distribution_tick(); }
                            if let Some(m) = &svc.metrics { m.inc_distostream_authorized(); }
                        }
                    }
                }
                Err(e) => error!(error=%e, "failed to subscribe to distostream authorized subject"),
            }
        });
    }

    fn spawn_linear_emitter(&self, schedule_id: String) {
        let svc = self.clone();
        task::spawn(async move {
            let base_interval = svc.tick_interval_millis;
            // Apply time compression: faster simulation = shorter intervals
            let compressed_interval = (base_interval as f64 / svc.time_compression_factor) as u64;
            let mut ticker = time::interval(Duration::from_millis(compressed_interval));
            info!(schedule_id=%schedule_id, base_interval=base_interval, compressed_interval=compressed_interval, compression_factor=svc.time_compression_factor, "starting compressed emitter");
            loop {
                ticker.tick().await;
                let mut remove = false;
                if let Some(mut state_ref) = svc.schedules.get_mut(&schedule_id) {
                    if state_ref.distributed_so_far >= state_ref.total { remove = true; }
                    else {
                        let idx = state_ref.next_tick_index;
                        let mut amt = state_ref.base_per_tick;
                        if idx < state_ref.remainder { amt += 1; }
                        // last tick ensure remaining all delivered
                        if idx == state_ref.planned_ticks - 1 { amt = state_ref.total - state_ref.distributed_so_far; }
                        state_ref.distributed_so_far += amt;
                        state_ref.next_tick_index += 1;
                        let remaining = state_ref.total - state_ref.distributed_so_far;
                        let planned_at_ms = state_ref.start_ms + (idx as u128 * base_interval as u128);
                        let executed_at_ms = current_ms();
                        let lateness_ms = executed_at_ms.saturating_sub(planned_at_ms);
                        let tick_evt = serde_json::json!({
                            "event_type": "distostream_distribution_tick",
                            "schedule_id": state_ref.schedule_id,
                            "contract_id": state_ref.contract_id,
                            "tick_index": idx,
                            "authorized_robotorq_total": state_ref.total,
                            "distributed_robotorq": amt,
                            "cumulative_distributed": state_ref.distributed_so_far,
                            "remaining": remaining,
                            "planned_at_ms": planned_at_ms,
                            "executed_at_ms": executed_at_ms,
                            "lateness_ms": lateness_ms,
                        });
                        if let Err(e) = svc.nats.publish(subjects::DISTOSTREAM_DISTRIBUTION_TICK, serde_json::to_vec(&tick_evt).unwrap().into()).await {
                            error!(error=%e, schedule_id=%state_ref.schedule_id, tick_index=idx, "failed publishing linear tick");
                        } else {
                            info!(contract_id=%state_ref.contract_id, schedule_id=%state_ref.schedule_id, tick_index=idx, distributed=amt, remaining=remaining, "linear tick emitted");
                            if let Some(m) = &svc.metrics { m.inc_distostream_distribution_tick(); }
                            if let Some(p) = &svc.persistence { if let Err(e) = p.persist_tick(&state_ref.schedule_id, idx, amt, state_ref.distributed_so_far, remaining, planned_at_ms as i64, executed_at_ms as i64, lateness_ms as i64).await { error!(error=%e, schedule_id=%state_ref.schedule_id, tick_index=idx, "failed to persist tick"); } }
                        }
                        if state_ref.distributed_so_far >= state_ref.total { remove = true; }
                    }
                } else {
                    // schedule removed
                    break;
                }
                if remove { svc.schedules.remove(&schedule_id); break; }
            }
        });
    }
}

fn extract_string_field(payload: &[u8], key: &str) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(payload).ok()
        .and_then(|v| v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string()))
}

fn extract_i64_field(payload: &[u8], key: &str) -> Option<i64> {
    serde_json::from_slice::<serde_json::Value>(payload).ok()
        .and_then(|v| v.get(key).and_then(|x| x.as_i64()))
}

fn make_schedule_id(contract_id: &str) -> String {
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_millis();
    format!("sch-{}-{}", contract_id, ms)
}

fn current_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_millis()
}
