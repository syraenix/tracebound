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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::RunId,
        engine,
        scenario::{Scenario, ScenarioRegistry},
    };
    use std::path::PathBuf;
    use uuid::Uuid;

    fn scenario() -> Scenario {
        ScenarioRegistry::load_embedded()
            .unwrap()
            .get("context-vault")
            .unwrap()
            .clone()
    }

    fn database() -> (PathBuf, String) {
        let path = std::env::temp_dir().join(format!("tracebound-test-{}.sqlite3", Uuid::new_v4()));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        (path, url)
    }

    #[tokio::test]
    async fn persisted_run_reopens_after_store_restart() {
        let (path, url) = database();
        let id = RunId("restart-recovery".into());
        {
            let store = Store::connect(&url).await.unwrap();
            let state = engine::new_run(id.clone(), &scenario());
            store.create_run(&state).await.unwrap();
        }
        let reopened = Store::connect(&url).await.unwrap();
        let restored = reopened.load_run(&id).await.unwrap();
        assert_eq!(restored.id, id);
        assert_eq!(restored.focus_remaining, scenario().focus_budget);
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn run_and_trace_update_roll_back_together() {
        let (path, url) = database();
        let store = Store::connect(&url).await.unwrap();
        let scenario = scenario();
        let id = RunId("atomic-update".into());
        let mut state = engine::new_run(id.clone(), &scenario);
        store.create_run(&state).await.unwrap();

        state.status = crate::domain::RunStatus::Ready;
        store.save_run(&state, &[]).await.unwrap();
        let before_failed_save = state.clone();

        let mut invalid_update = state.clone();
        invalid_update.focus_remaining = 1;
        let duplicate = crate::domain::TraceEvent {
            id: crate::domain::TraceEventId("duplicate".into()),
            run_id: id.clone(),
            sequence: 1,
            elapsed_ms: 0,
            actor: "test".into(),
            kind: crate::domain::TraceKind::WarningRaised,
            summary: "duplicate".into(),
            payload: serde_json::json!({}),
            focus_delta: 0,
            related_state_keys: vec![],
        };
        store
            .save_run(&state, std::slice::from_ref(&duplicate))
            .await
            .unwrap();
        let error = store
            .save_run(&invalid_update, &[duplicate])
            .await
            .unwrap_err();
        assert!(matches!(error, DomainError::Persistence(_)));

        let restored = store.load_run(&id).await.unwrap();
        assert_eq!(restored.focus_remaining, before_failed_save.focus_remaining);
        assert_eq!(store.trace_after(&id, 0).await.unwrap().len(), 1);
        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
