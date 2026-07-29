use crate::domain::{DomainError, ScenarioId};
use rust_embed::Embed;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Embed)]
#[folder = "scenarios/"]
struct ScenarioAssets;

#[derive(Clone, Debug, Deserialize)]
pub struct Scenario {
    pub schema_version: u32,
    pub id: ScenarioId,
    pub title: String,
    pub sequence: u32,
    pub difficulty: u32,
    pub focus_budget: i32,
    pub context_capacity: i32,
    pub learning_principle: String,
    pub codex_title: String,
    pub briefing: Briefing,
    pub initial_state: BTreeMap<String, bool>,
    pub context: Vec<ContextItem>,
    pub rules: Vec<Rule>,
    pub graders: Vec<Grader>,
    pub postmortem: PostmortemText,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Briefing {
    pub summary: String,
    pub objective: String,
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ContextItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cost: i32,
    pub trust: String,
    pub freshness: String,
    pub effects: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Rule {
    pub id: String,
    pub requires_all_state: Vec<String>,
    pub forbids_state: Vec<String>,
    pub set: BTreeMap<String, bool>,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraderKind {
    StateEquals,
    FocusAtLeast,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Grader {
    pub id: String,
    pub description: String,
    pub kind: GraderKind,
    pub path: Option<String>,
    pub expected: Option<bool>,
    pub expected_number: Option<i32>,
    pub weight: u32,
    pub critical: bool,
    pub failure: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostmortemText {
    pub success: String,
    pub failure: String,
    pub alternative: String,
}

#[derive(Clone)]
pub struct ScenarioRegistry {
    scenarios: HashMap<String, Scenario>,
}

impl ScenarioRegistry {
    pub fn load_embedded() -> Result<Self, DomainError> {
        let mut scenarios = HashMap::new();
        for path in ScenarioAssets::iter().filter(|p| p.ends_with("scenario.toml")) {
            let bytes = ScenarioAssets::get(&path)
                .ok_or_else(|| DomainError::InvalidScenario(path.to_string()))?;
            let text = std::str::from_utf8(&bytes.data)
                .map_err(|error| DomainError::InvalidScenario(error.to_string()))?;
            let scenario: Scenario = toml::from_str(text)
                .map_err(|error| DomainError::InvalidScenario(error.to_string()))?;
            validate(&scenario)?;
            scenarios.insert(scenario.id.0.clone(), scenario);
        }
        if scenarios.is_empty() {
            return Err(DomainError::InvalidScenario("no cartridges found".into()));
        }
        Ok(Self { scenarios })
    }

    pub fn get(&self, id: &str) -> Result<&Scenario, DomainError> {
        self.scenarios
            .get(id)
            .ok_or_else(|| DomainError::ScenarioNotFound(id.into()))
    }
}

pub fn validate(scenario: &Scenario) -> Result<(), DomainError> {
    if scenario.schema_version != 1 {
        return Err(DomainError::InvalidScenario(
            "unsupported schema version".into(),
        ));
    }
    if scenario.id.0.trim().is_empty() || scenario.title.trim().is_empty() {
        return Err(DomainError::InvalidScenario(
            "id and title are required".into(),
        ));
    }
    if scenario.focus_budget <= 0 || scenario.context_capacity <= 0 {
        return Err(DomainError::InvalidScenario(
            "budgets must be positive".into(),
        ));
    }
    let mut ids = HashSet::new();
    for item in &scenario.context {
        if item.cost < 0 || !ids.insert(&item.id) {
            return Err(DomainError::InvalidScenario(format!(
                "invalid or duplicate context id: {}",
                item.id
            )));
        }
    }
    if scenario.graders.is_empty() || scenario.graders.iter().map(|g| g.weight).sum::<u32>() != 100
    {
        return Err(DomainError::InvalidScenario(
            "grader weights must total 100".into(),
        ));
    }
    for grader in &scenario.graders {
        if matches!(grader.kind, GraderKind::StateEquals) {
            let path = grader.path.as_ref().ok_or_else(|| {
                DomainError::InvalidScenario(format!("grader {} needs a path", grader.id))
            })?;
            if !scenario.initial_state.contains_key(path) {
                return Err(DomainError::InvalidScenario(format!(
                    "grader {} references unknown state {path}",
                    grader.id
                )));
            }
        }
    }
    Ok(())
}
