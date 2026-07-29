use crate::{
    domain::{
        ContextId, DomainError, GraderResult, Loadout, PendingDecision, Postmortem, RunId,
        RunOutcome, RunState, RunStatus, TraceEvent, TraceEventId, TraceKind,
    },
    scenario::{GraderKind, Scenario},
};
use serde_json::json;
use std::collections::HashSet;

pub fn new_run(id: RunId, scenario: &Scenario) -> RunState {
    RunState {
        id,
        scenario_id: scenario.id.clone(),
        status: RunStatus::Configuring,
        focus_remaining: scenario.focus_budget,
        loadout: None,
        world_state: scenario.initial_state.clone(),
        next_rule: 0,
        trace_sequence: 0,
        pending_decision: None,
        outcome: None,
        score: None,
        graders: vec![],
    }
}

pub fn configure(
    state: &mut RunState,
    scenario: &Scenario,
    loadout: Loadout,
) -> Result<(), DomainError> {
    if state.status != RunStatus::Configuring {
        return Err(DomainError::InvalidTransition(
            "run is not configuring".into(),
        ));
    }
    let unique: HashSet<_> = loadout.context.iter().collect();
    if unique.len() != loadout.context.len() {
        return Err(DomainError::InvalidLoadout(
            "context cannot be selected twice".into(),
        ));
    }
    let cost = context_cost(scenario, &loadout.context)?;
    if cost > scenario.context_capacity {
        return Err(DomainError::InvalidLoadout(format!(
            "selected context costs {cost}; capacity is {}",
            scenario.context_capacity
        )));
    }
    state.loadout = Some(loadout);
    state.status = RunStatus::Ready;
    Ok(())
}

pub fn start(state: &mut RunState, scenario: &Scenario) -> Result<Vec<TraceEvent>, DomainError> {
    if state.status != RunStatus::Ready {
        return Err(DomainError::InvalidTransition("run is not ready".into()));
    }
    let loadout = state.loadout.clone().expect("ready run has a loadout");
    let mut events = vec![];
    emit(
        state,
        &mut events,
        TraceKind::RunStarted,
        "Expedition started.",
        0,
        vec![],
    );
    for selected in &loadout.context {
        let item = scenario
            .context
            .iter()
            .find(|item| item.id == selected.0)
            .expect("loadout was validated");
        state.focus_remaining -= item.cost;
        for (key, value) in &item.effects {
            state.world_state.insert(key.clone(), *value);
        }
        emit(
            state,
            &mut events,
            TraceKind::ContextLoaded,
            format!(
                "Loaded {} [{} trust, {}].",
                item.title, item.trust, item.freshness
            ),
            -item.cost,
            item.effects.keys().cloned().collect(),
        );
    }
    emit(
        state,
        &mut events,
        TraceKind::PlanFormed,
        "Plan: compare current symptoms with recent system changes.",
        0,
        vec![],
    );
    state.status = RunStatus::AwaitingDecision;
    state.pending_decision = Some(PendingDecision {
        id: "evidence-policy".into(),
        prompt: "Should the agent privilege current corroborated evidence over authored procedure?"
            .into(),
    });
    emit(
        state,
        &mut events,
        TraceKind::DecisionRequested,
        "Evidence conflict policy required.",
        0,
        vec![],
    );
    Ok(events)
}

pub fn decide(
    state: &mut RunState,
    scenario: &Scenario,
    prefer_current: bool,
) -> Result<Vec<TraceEvent>, DomainError> {
    if state.status != RunStatus::AwaitingDecision || state.pending_decision.is_none() {
        return Err(DomainError::DecisionNotPending);
    }
    state.pending_decision = None;
    state.status = RunStatus::Running;
    if prefer_current {
        state.world_state.insert("misled_by_runbook".into(), false);
    }
    let mut events = vec![];
    emit(
        state,
        &mut events,
        TraceKind::DecisionResolved,
        if prefer_current {
            "Current corroborated evidence was given priority."
        } else {
            "Authored procedure was followed despite freshness concerns."
        },
        0,
        vec!["misled_by_runbook".into()],
    );
    advance(state, scenario, &mut events);
    Ok(events)
}

fn advance(state: &mut RunState, scenario: &Scenario, events: &mut Vec<TraceEvent>) {
    for rule in &scenario.rules {
        let eligible = rule
            .requires_all_state
            .iter()
            .all(|key| state.world_state.get(key) == Some(&true))
            && rule
                .forbids_state
                .iter()
                .all(|key| state.world_state.get(key) != Some(&true));
        if eligible {
            for (key, value) in &rule.set {
                state.world_state.insert(key.clone(), *value);
            }
            emit(
                state,
                events,
                if rule.set.is_empty() {
                    TraceKind::WarningRaised
                } else {
                    TraceKind::StateChanged
                },
                rule.summary.clone(),
                -1,
                rule.set.keys().cloned().collect(),
            );
            state.focus_remaining -= 1;
        }
    }
    emit(
        state,
        events,
        TraceKind::AgentClaimedCompletion,
        "Investigation complete; submitting findings to objective graders.",
        0,
        vec![],
    );
    finish(state, scenario, events);
}

fn finish(state: &mut RunState, scenario: &Scenario, events: &mut Vec<TraceEvent>) {
    let results: Vec<_> = scenario
        .graders
        .iter()
        .map(|grader| {
            let passed = match grader.kind {
                GraderKind::StateEquals => {
                    state.world_state.get(grader.path.as_ref().unwrap()) == grader.expected.as_ref()
                }
                GraderKind::FocusAtLeast => {
                    state.focus_remaining >= grader.expected_number.unwrap_or_default()
                }
            };
            GraderResult {
                id: grader.id.clone(),
                description: grader.description.clone(),
                passed,
                weight: grader.weight,
                critical: grader.critical,
                explanation: if passed {
                    "Passed.".into()
                } else {
                    grader.failure.clone()
                },
            }
        })
        .collect();
    let score = results.iter().filter(|r| r.passed).map(|r| r.weight).sum();
    let critical_failed = results.iter().any(|r| r.critical && !r.passed);
    let outcome = if state.focus_remaining < 0 {
        RunOutcome::BudgetExhausted
    } else if critical_failed {
        RunOutcome::SafeFailure
    } else if score == 100 {
        RunOutcome::Success
    } else {
        RunOutcome::QualifiedSuccess
    };
    for result in &results {
        emit(
            state,
            events,
            TraceKind::EvaluatorExecuted,
            format!(
                "{}: {}",
                result.description,
                if result.passed { "passed" } else { "failed" }
            ),
            0,
            vec![],
        );
    }
    state.status = if matches!(outcome, RunOutcome::Success | RunOutcome::QualifiedSuccess) {
        RunStatus::Completed
    } else {
        RunStatus::Failed
    };
    state.score = Some(score);
    state.outcome = Some(outcome.clone());
    state.graders = results;
    emit(
        state,
        events,
        TraceKind::RunEnded,
        format!("Expedition ended: {outcome:?} ({score}/100)."),
        0,
        vec![],
    );
}

pub fn postmortem(
    state: &RunState,
    scenario: &Scenario,
    events: &[TraceEvent],
) -> Result<Postmortem, DomainError> {
    let outcome = state
        .outcome
        .clone()
        .ok_or_else(|| DomainError::InvalidTransition("run has not ended".into()))?;
    let success = matches!(outcome, RunOutcome::Success | RunOutcome::QualifiedSuccess);
    Ok(Postmortem {
        outcome,
        score: state.score.unwrap_or_default(),
        summary: if success {
            scenario.postmortem.success.clone()
        } else {
            scenario.postmortem.failure.clone()
        },
        critical_decision: if state.world_state.get("misled_by_runbook") == Some(&true) {
            "Allowed a stale runbook to steer the investigation.".into()
        } else {
            "Privileged current, corroborated evidence.".into()
        },
        first_warning: events
            .iter()
            .find(|event| event.kind == TraceKind::WarningRaised)
            .map(|event| event.summary.clone())
            .unwrap_or_else(|| "No warning was raised before grading.".into()),
        alternative: scenario.postmortem.alternative.clone(),
        learning_principle: scenario.learning_principle.clone(),
        graders: state.graders.clone(),
    })
}

fn context_cost(scenario: &Scenario, selected: &[ContextId]) -> Result<i32, DomainError> {
    selected.iter().try_fold(0, |total, id| {
        scenario
            .context
            .iter()
            .find(|item| item.id == id.0)
            .map(|item| total + item.cost)
            .ok_or_else(|| DomainError::InvalidLoadout(format!("unknown context: {id}")))
    })
}

fn emit(
    state: &mut RunState,
    events: &mut Vec<TraceEvent>,
    kind: TraceKind,
    summary: impl Into<String>,
    focus_delta: i32,
    related_state_keys: Vec<String>,
) {
    state.trace_sequence += 1;
    events.push(TraceEvent {
        id: TraceEventId(format!("{}-{}", state.id, state.trace_sequence)),
        run_id: state.id.clone(),
        sequence: state.trace_sequence,
        elapsed_ms: state.trace_sequence * 100,
        actor: "harness".into(),
        kind,
        summary: summary.into(),
        payload: json!({}),
        focus_delta,
        related_state_keys,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{ApprovalPolicy, AutonomyLevel},
        scenario::ScenarioRegistry,
    };

    fn scenario() -> Scenario {
        ScenarioRegistry::load_embedded()
            .unwrap()
            .get("context-vault")
            .unwrap()
            .clone()
    }

    fn loadout(ids: &[&str]) -> Loadout {
        Loadout {
            context: ids.iter().map(|id| ContextId((*id).into())).collect(),
            autonomy: AutonomyLevel::Guided,
            approval_policy: ApprovalPolicy::HighRisk,
        }
    }

    #[test]
    fn golden_current_evidence_run_succeeds_deterministically() {
        let scenario = scenario();
        let execute = || {
            let mut state = new_run(RunId("golden-success".into()), &scenario);
            configure(
                &mut state,
                &scenario,
                loadout(&["recent-logs", "deployment-note"]),
            )
            .unwrap();
            let mut events = start(&mut state, &scenario).unwrap();
            events.extend(decide(&mut state, &scenario, true).unwrap());
            (state, events)
        };
        let (first, first_events) = execute();
        let (second, second_events) = execute();
        assert_eq!(first.outcome, Some(RunOutcome::Success));
        assert_eq!(first.score, Some(100));
        assert_eq!(
            first_events
                .iter()
                .map(|event| (&event.kind, &event.summary))
                .collect::<Vec<_>>(),
            second_events
                .iter()
                .map(|event| (&event.kind, &event.summary))
                .collect::<Vec<_>>()
        );
        assert_eq!(first.world_state, second.world_state);
    }

    #[test]
    fn golden_stale_context_run_fails_critical_grader() {
        let scenario = scenario();
        let mut state = new_run(RunId("golden-failure".into()), &scenario);
        configure(&mut state, &scenario, loadout(&["obsolete-runbook"])).unwrap();
        start(&mut state, &scenario).unwrap();
        let events = decide(&mut state, &scenario, false).unwrap();
        assert_eq!(state.outcome, Some(RunOutcome::SafeFailure));
        assert!(
            state
                .graders
                .iter()
                .any(|grader| grader.critical && !grader.passed)
        );
        assert!(
            events
                .iter()
                .any(|event| event.kind == TraceKind::WarningRaised)
        );
    }

    #[test]
    fn context_capacity_is_enforced() {
        let scenario = scenario();
        let mut state = new_run(RunId("capacity".into()), &scenario);
        let error = configure(
            &mut state,
            &scenario,
            loadout(&["recent-logs", "deployment-note", "architecture-map"]),
        )
        .unwrap_err();
        assert!(matches!(error, DomainError::InvalidLoadout(_)));
        assert_eq!(state.status, RunStatus::Configuring);
    }
}
