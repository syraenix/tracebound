# Tracebound: The Harness Below

> Build the harness. Read the trace. Survive the outcome.

**Tracebound** is a local-first educational roguelike that teaches AI-native software development by turning context, tools, approvals, traces, budgets, and evaluations into game mechanics.

Instead of directly controlling a hero, the player configures and supervises an AI engineering harness attempting simulated software-delivery missions. Success depends less on writing a clever prompt and more on creating an environment where imperfect intelligence can act safely, leave evidence, recover from mistakes, and be judged against reality.

> **Project status:** Early development. The first deterministic, single-player vertical slice is playable locally without an LLM or API key.

## Why Tracebound?

Many important lessons in AI-native engineering are experiential:

- More context can make an agent less effective.
- Broad tools can create ambiguous or dangerous behavior.
- Persuasive output is not proof that a task was completed.
- Autonomy without observability and recovery is a liability.
- Multi-agent systems introduce coordination costs as well as capability.
- Objective evaluations matter more than confident completion claims.

Documentation can explain these ideas. Tracebound lets players experience their consequences, retry quickly, and compare approaches with teammates.

## Core Gameplay Loop

Each encounter follows a compact roguelike expedition:

1. Receive an ambiguous engineering objective.
2. Assemble a harness from instructions, context, tools, permissions, approval policies, budgets, and evaluators.
3. Run a deterministic agent simulation against a fictional software environment.
4. Inspect the execution trace and respond to intervention points.
5. Reach a success, qualified success, or failure state.
6. Review a deterministic postmortem explaining what happened and why.
7. Unlock a short codex entry and replay with a different strategy.

A complete encounter should take approximately 15 to 25 minutes.

## The Mechanic Is the Lesson

| Roguelike concept | AI-native engineering concept |
|---|---|
| Quest | Engineering objective |
| Inventory | Available context |
| Abilities | Tools and APIs |
| Focus | Token, latency, and cost budget |
| Wards | Guardrails and approval policies |
| Expedition log | Execution trace |
| Traps | Stale context, prompt injection, and unsafe tool use |
| Campfire | Checkpoint and durable memory |
| Party members | Specialized agents |
| Victory conditions | Objective graders |
| Postmortem | Harness review and learning summary |

Tracebound is not intended to hide a quiz beneath fantasy decoration. Player decisions should create consequences that reflect real engineering tradeoffs.

## Learning Objectives

The MVP campaign is designed to teach players how to:

- Define observable success before starting execution.
- Select relevant and trustworthy context.
- Design narrow, legible tools with appropriate permissions.
- Match agent autonomy to operational risk.
- Inspect traces instead of trusting summaries.
- Evaluate actual environment state with repeatable graders.
- Preserve progress through checkpoints and handoff artifacts.
- Add specialized agents only when their value exceeds coordination cost.

## MVP Campaign

The first campaign is planned as six sequential encounters:

1. **The Mines of Ambiguity**

   Define measurable success before execution begins.

2. **The Context Vault**

   Choose relevant context while avoiding stale and misleading material.

3. **The Toolsmith's Forge**

   Compare broad capabilities with narrow, purpose-built tools.

4. **The Hall of Mirrors**

   Inspect traces and reject unsupported completion claims.

5. **The Memory Marsh**

   Preserve durable state across long-running work.

6. **The Guild of Too Many Agents**

   Balance specialization against routing and coordination overhead.

## Design Principles

- **Deterministic truth, variable presentation.** Scenario state and grading remain repeatable. Optional AI narration may vary, but it cannot change facts or outcomes.
- **Failure should reveal structure.** A failed run should explain the missing safeguard, misleading evidence, or harmful decision.
- **Start simple, earn autonomy.** Deterministic workflows should remain preferable until additional autonomy provides measurable value.
- **Local-first and inspectable.** The game should work offline after installation, with readable scenarios, traces, and saved state.
- **Short sessions, dense consequences.** Encounters should contain a small number of meaningful decisions rather than filler.
- **Retro atmosphere, modern usability.** DOS and Amiga-inspired presentation must not compromise accessibility or trace readability.

## Planned Technical Stack

The MVP is designed as a self-contained local web application:

- **Rust** for the application and deterministic simulation engine
- **Axum** for HTTP routing
- **Askama** for server-rendered templates
- **htmx** for HTML fragment interactions
- **Server-Sent Events** for live trace updates
- **SQLite** for local campaign and run state
- **TOML** for declarative scenario cartridges
- Embedded templates and static assets for single-executable distribution

No external model provider is required for the MVP.

## Getting Started

Build and run the current vertical slice with:

```bash
git clone git@github.com:syraenix/tracebound.git
cd tracebound
cargo run
```

The application will then be available from a local browser URL printed by the server.

The first playable encounter is **The Context Vault**. See the product requirements
document and [MVP checklist](docs/mvp-checklist.md) for the implementation boundary.

## Scenario Cartridges

Adventures are intended to be authored primarily as data rather than custom engine code.

A scenario cartridge will define:

- The quest briefing and learning objective
- Available context items
- Tools, permissions, risks, and deterministic effects
- Agent instructions and autonomy options
- Approval and intervention points
- Hidden scenario state
- Weighted graders
- Postmortem rules
- Codex unlocks

Illustrative shape:

```toml
id = "phantom-inventory"
title = "The Phantom Inventory"
difficulty = 2

concepts = [
  "context-selection",
  "tool-permissions",
  "regression-evals",
]

[objective]
description = "Identify and fix the overselling defect."
success_state = "race_condition_fixed"

[[tools]]
id = "query-staging-db"
risk = "low"
requires_approval = false

[[tools]]
id = "modify-production-inventory"
risk = "critical"
requires_approval = true

[[graders]]
type = "state"
assertion = "race_condition_fixed"

[[graders]]
type = "forbidden_action"
action = "modify-production-inventory"
```

The final cartridge schema will evolve alongside the first playable encounter.

## Safety and Privacy

The MVP operates entirely within simulated environments.

It will not:

- Execute arbitrary shell commands
- Read or modify the player's real filesystem
- Access source repositories, cloud accounts, or company systems
- Send traces to an external service by default
- Require an API key or model-provider account

Future model integrations must remain optional and must not control scenario truth or grading.

## Initial Delivery Plan

1. Scaffold the Rust application and local persistence.
2. Implement the deterministic run state machine.
3. Build one complete vertical-slice encounter.
4. Add trace streaming, interventions, grading, and postmortems.
5. Generalize the scenario cartridge format.
6. Complete the six-encounter MVP campaign.
7. Add shareable Markdown expedition reports.
8. Explore an optional AI narrator and coach behind a provider interface.

The first milestone should prove the entire gameplay loop with one polished encounter before expanding campaign breadth.

## Project Documentation

- [`docs/tracebound_prd.md`](docs/tracebound_prd.md) contains the product and technical requirements.
- GitHub issues should capture implementation work, design decisions, and scenario proposals.
- Architectural decisions should be recorded as lightweight ADRs when they establish lasting constraints.

## Contributing

Tracebound is at an early stage, so focused contributions are more useful than broad rewrites.

Before beginning a substantial change:

1. Open or comment on an issue describing the intended outcome.
2. Keep engine changes separate from scenario-content changes where practical.
3. Prefer a working vertical slice over speculative abstraction.
4. Add deterministic tests for simulation and grading behavior.
5. Preserve the local-first, no-required-LLM experience.

Scenario contributions should teach through consequences rather than quiz questions or renamed documentation.

## License

Tracebound is licensed under the MIT License. See [`LICENSE`](LICENSE).
