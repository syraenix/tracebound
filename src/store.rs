use crate::domain::{DomainError, RunId, RunState, TraceEvent};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn connect(url: &str) -> Result<Self, DomainError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(persistence)?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(persistence)?;
        Ok(Self { pool })
    }

    pub async fn create_run(&self, state: &RunState) -> Result<(), DomainError> {
        sqlx::query("INSERT INTO runs (id, scenario_id, status, state_json) VALUES (?, ?, ?, ?)")
            .bind(&state.id.0)
            .bind(&state.scenario_id.0)
            .bind(status_name(state))
            .bind(serde_json::to_string(state).map_err(persistence)?)
            .execute(&self.pool)
            .await
            .map_err(persistence)?;
        Ok(())
    }

    pub async fn load_run(&self, id: &RunId) -> Result<RunState, DomainError> {
        let row = sqlx::query("SELECT state_json FROM runs WHERE id = ?")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(persistence)?
            .ok_or_else(|| DomainError::RunNotFound(id.0.clone()))?;
        serde_json::from_str(row.get("state_json")).map_err(persistence)
    }

    pub async fn save_run(
        &self,
        state: &RunState,
        events: &[TraceEvent],
    ) -> Result<(), DomainError> {
        let mut transaction = self.pool.begin().await.map_err(persistence)?;
        sqlx::query(
            "UPDATE runs SET status = ?, state_json = ?, score = ?, outcome = ?, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(status_name(state))
        .bind(serde_json::to_string(state).map_err(persistence)?)
        .bind(state.score.map(i64::from))
        .bind(state.outcome.as_ref().map(|value| format!("{value:?}")))
        .bind(&state.id.0)
        .execute(&mut *transaction)
        .await
        .map_err(persistence)?;
        for event in events {
            sqlx::query(
                "INSERT INTO trace_events (id, run_id, sequence, event_json) VALUES (?, ?, ?, ?)",
            )
            .bind(&event.id.0)
            .bind(&event.run_id.0)
            .bind(event.sequence as i64)
            .bind(serde_json::to_string(event).map_err(persistence)?)
            .execute(&mut *transaction)
            .await
            .map_err(persistence)?;
        }
        transaction.commit().await.map_err(persistence)?;
        Ok(())
    }

    pub async fn trace_after(
        &self,
        id: &RunId,
        sequence: u64,
    ) -> Result<Vec<TraceEvent>, DomainError> {
        let rows = sqlx::query(
            "SELECT event_json FROM trace_events WHERE run_id = ? AND sequence > ? \
             ORDER BY sequence",
        )
        .bind(&id.0)
        .bind(sequence as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(persistence)?;
        rows.into_iter()
            .map(|row| serde_json::from_str(row.get("event_json")).map_err(persistence))
            .collect()
    }
}

fn status_name(state: &RunState) -> String {
    format!("{:?}", state.status).to_lowercase()
}

fn persistence(error: impl std::fmt::Display) -> DomainError {
    DomainError::Persistence(error.to_string())
}
