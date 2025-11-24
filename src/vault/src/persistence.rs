use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, anyhow};
use deadpool_postgres::{Manager, Pool};
use tokio_postgres::{NoTls, Row};
use tracing::{info, error};

#[derive(Clone)]
pub struct VaultPersistence {
    pool: Pool,
}

#[derive(Clone, Debug)]
pub struct PersistedSchedule {
    pub schedule_id: String,
    pub contract_id: String,
    pub total: i64,
    pub planned_ticks: i64,
    pub base_per_tick: i64,
    pub remainder: i64,
    pub next_tick_index: i64,
    pub distributed_so_far: i64,
    pub start_ms: i64,
}

impl VaultPersistence {
    pub async fn new(db_url: &str) -> Result<Self> {
        let mgr = Manager::new(db_url.parse()?, NoTls);
        let pool = Pool::builder(mgr).max_size(5).build().unwrap();
        let inst = Self { pool };
        inst.init_schema().await?;
        Ok(inst)
    }

    async fn init_schema(&self) -> Result<()> {
        let client = self.pool.get().await?;
        client.batch_execute(r#"
        CREATE TABLE IF NOT EXISTS disto_schedules (
            schedule_id TEXT PRIMARY KEY,
            contract_id TEXT NOT NULL,
            total BIGINT NOT NULL,
            planned_ticks BIGINT NOT NULL,
            base_per_tick BIGINT NOT NULL,
            remainder BIGINT NOT NULL,
            next_tick_index BIGINT NOT NULL,
            distributed_so_far BIGINT NOT NULL,
            start_ms BIGINT NOT NULL,
            completed BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE TABLE IF NOT EXISTS disto_ticks (
            id BIGSERIAL PRIMARY KEY,
            schedule_id TEXT NOT NULL REFERENCES disto_schedules(schedule_id) ON DELETE CASCADE,
            tick_index BIGINT NOT NULL,
            distributed BIGINT NOT NULL,
            cumulative BIGINT NOT NULL,
            remaining BIGINT NOT NULL,
            planned_at_ms BIGINT NOT NULL,
            executed_at_ms BIGINT NOT NULL,
            lateness_ms BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#).await?;
        Ok(())
    }

    pub async fn persist_schedule(&self, s: &PersistedSchedule) -> Result<()> {
        let client = self.pool.get().await?;
        client.execute(r#"
            INSERT INTO disto_schedules(schedule_id, contract_id, total, planned_ticks, base_per_tick, remainder, next_tick_index, distributed_so_far, start_ms, completed)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,FALSE)
            ON CONFLICT (schedule_id) DO NOTHING
        "#, &[
            &s.schedule_id,
            &s.contract_id,
            &s.total,
            &s.planned_ticks,
            &s.base_per_tick,
            &s.remainder,
            &s.next_tick_index,
            &s.distributed_so_far,
            &s.start_ms,
        ]).await?;
        Ok(())
    }

    pub async fn persist_tick(&self, schedule_id: &str, tick_index: i64, distributed: i64, cumulative: i64, remaining: i64, planned_at_ms: i64, executed_at_ms: i64, lateness_ms: i64) -> Result<()> {
        let client = self.pool.get().await?;
        client.execute(r#"
            INSERT INTO disto_ticks(schedule_id, tick_index, distributed, cumulative, remaining, planned_at_ms, executed_at_ms, lateness_ms)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
        "#, &[
            &schedule_id,
            &tick_index,
            &distributed,
            &cumulative,
            &remaining,
            &planned_at_ms,
            &executed_at_ms,
            &lateness_ms,
        ]).await?;
        // update schedule state
        client.execute(r#"
            UPDATE disto_schedules SET
                next_tick_index = $2,
                distributed_so_far = $3,
                completed = CASE WHEN $3 >= total THEN TRUE ELSE completed END
            WHERE schedule_id = $1
        "#, &[
            &schedule_id,
            &(tick_index + 1),
            &cumulative,
        ]).await?;
        Ok(())
    }

    pub async fn load_active_schedules(&self) -> Result<Vec<PersistedSchedule>> {
        let client = self.pool.get().await?;
        let rows = client.query(r#"
            SELECT schedule_id, contract_id, total, planned_ticks, base_per_tick, remainder, next_tick_index, distributed_so_far, start_ms
            FROM disto_schedules WHERE completed = FALSE
        "#, &[]).await?;
        Ok(rows.into_iter().map(row_to_schedule).collect())
    }
}

fn row_to_schedule(r: Row) -> PersistedSchedule {
    PersistedSchedule {
        schedule_id: r.get(0),
        contract_id: r.get(1),
        total: r.get(2),
        planned_ticks: r.get(3),
        base_per_tick: r.get(4),
        remainder: r.get(5),
        next_tick_index: r.get(6),
        distributed_so_far: r.get(7),
        start_ms: r.get(8),
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}
