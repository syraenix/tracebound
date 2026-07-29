use crate::{
    domain::{
        ApprovalPolicy, AutonomyLevel, ContextId, DomainError, Loadout, Postmortem, RunId,
        RunState, TraceEvent,
    },
    engine,
    scenario::{Scenario, ScenarioRegistry},
    store::Store,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppService {
    pub store: Store,
    pub scenarios: ScenarioRegistry,
}

impl AppService {
    pub fn scenario(&self, id: &str) -> Result<&Scenario, DomainError> {
        self.scenarios.get(id)
    }

    pub async fn create_run(&self, scenario_id: &str) -> Result<RunState, DomainError> {
        let scenario = self.scenario(scenario_id)?;
        let state = engine::new_run(RunId(Uuid::new_v4().to_string()), scenario);
        self.store.create_run(&state).await?;
        Ok(state)
    }

    pub async fn load_run(&self, id: &str) -> Result<RunState, DomainError> {
        self.store.load_run(&RunId(id.into())).await
    }

    pub async fn configure(&self, id: &str, context: Vec<String>) -> Result<RunState, DomainError> {
        let mut state = self.load_run(id).await?;
        let scenario = self.scenario(&state.scenario_id.0)?;
        engine::configure(
            &mut state,
            scenario,
            Loadout {
                context: context.into_iter().map(ContextId).collect(),
                autonomy: AutonomyLevel::Guided,
                approval_policy: ApprovalPolicy::HighRisk,
            },
        )?;
        self.store.save_run(&state, &[]).await?;
        Ok(state)
    }

    pub async fn start(&self, id: &str) -> Result<RunState, DomainError> {
        let mut state = self.load_run(id).await?;
        let scenario = self.scenario(&state.scenario_id.0)?;
        let events = engine::start(&mut state, scenario)?;
        self.store.save_run(&state, &events).await?;
        Ok(state)
    }

    pub async fn decide(&self, id: &str, prefer_current: bool) -> Result<RunState, DomainError> {
        let mut state = self.load_run(id).await?;
        let scenario = self.scenario(&state.scenario_id.0)?;
        let events = engine::decide(&mut state, scenario, prefer_current)?;
        self.store.save_run(&state, &events).await?;
        Ok(state)
    }

    pub async fn trace(&self, id: &str, after: u64) -> Result<Vec<TraceEvent>, DomainError> {
        self.store.trace_after(&RunId(id.into()), after).await
    }

    pub async fn postmortem(&self, id: &str) -> Result<Postmortem, DomainError> {
        let state = self.load_run(id).await?;
        let scenario = self.scenario(&state.scenario_id.0)?;
        let events = self.trace(id, 0).await?;
        engine::postmortem(&state, scenario, &events)
    }
}
