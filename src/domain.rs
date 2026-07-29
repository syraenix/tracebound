use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(ScenarioId);
id_type!(RunId);
id_type!(ContextId);
id_type!(TraceEventId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Configuring,
    Ready,
    Running,
    AwaitingApproval,
    AwaitingDecision,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    Workflow,
    Guided,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    AllChanges,
    HighRisk,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Success,
    QualifiedSuccess,
    SafeFailure,
    BudgetExhausted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loadout {
    pub context: Vec<ContextId>,
    pub autonomy: AutonomyLevel,
    pub approval_policy: ApprovalPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingDecision {
    pub id: String,
    pub prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: String,
    pub action: String,
    pub risk: RiskLevel,
    pub inputs: String,
    pub side_effects: String,
    pub alternative: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunState {
    pub id: RunId,
    pub scenario_id: ScenarioId,
    pub status: RunStatus,
    pub focus_remaining: i32,
    pub loadout: Option<Loadout>,
    pub world_state: BTreeMap<String, bool>,
    pub next_rule: usize,
    pub trace_sequence: u64,
    pub pending_decision: Option<PendingDecision>,
    pub pending_approval: Option<PendingApproval>,
    pub outcome: Option<RunOutcome>,
    pub score: Option<u32>,
    pub graders: Vec<GraderResult>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceKind {
    RunStarted,
    ContextLoaded,
    PlanFormed,
    WarningRaised,
    DecisionRequested,
    DecisionResolved,
    ToolProposed,
    ApprovalRequested,
    ApprovalGranted,
    ApprovalDenied,
    ToolExecuted,
    StateChanged,
    EvaluatorExecuted,
    BudgetChanged,
    AgentClaimedCompletion,
    RunEnded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceEvent {
    pub id: TraceEventId,
    pub run_id: RunId,
    pub sequence: u64,
    pub elapsed_ms: u64,
    pub actor: String,
    pub kind: TraceKind,
    pub summary: String,
    pub payload: serde_json::Value,
    pub focus_delta: i32,
    pub related_state_keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraderResult {
    pub id: String,
    pub description: String,
    pub passed: bool,
    pub weight: u32,
    pub critical: bool,
    pub explanation: String,
}

#[derive(Clone, Debug)]
pub struct Postmortem {
    pub outcome: RunOutcome,
    pub score: u32,
    pub summary: String,
    pub critical_decision: String,
    pub first_warning: String,
    pub alternative: String,
    pub learning_principle: String,
    pub graders: Vec<GraderResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("scenario not found: {0}")]
    ScenarioNotFound(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),
    #[error("invalid loadout: {0}")]
    InvalidLoadout(String),
    #[error("scenario is invalid: {0}")]
    InvalidScenario(String),
    #[error("decision is no longer pending")]
    DecisionNotPending,
    #[error("approval is no longer pending")]
    ApprovalNotPending,
    #[error("persistence failed: {0}")]
    Persistence(String),
}
