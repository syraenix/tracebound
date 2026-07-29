# Vertical Slice Architecture

The first Tracebound increment is a single crate with explicit module boundaries:

```text
Browser → web → app → engine → domain
                    ↘ store
Scenario TOML → scenario registry → engine
```

- `domain` owns identifiers, run state, loadouts, trace events, graders, and errors.
- `scenario` loads and validates declarative TOML cartridges.
- `engine` is deterministic and has no HTTP, HTML, SQLite, clock, or model dependency.
- `store` owns SQLite migrations and atomic run-plus-trace persistence.
- `app` coordinates scenario, engine, and persistence operations.
- `web` maps HTTP requests to application operations and renders server-side HTML.

The crate can be split into the PRD’s proposed workspace without changing these
dependencies. Scenario effects are declarative. They never execute cartridge code,
shell commands, filesystem operations, or network requests.

## Initial domain model

A `RunState` contains a strongly typed run/scenario ID, lifecycle status, selected
context IDs, Focus, deterministic step cursor, world state, pending intervention,
trace sequence, and optional outcome. A `Scenario` contains loadout constraints,
context definitions, deterministic rules, graders, and authored postmortem/codex
content.

The engine accepts only a scenario, persisted run state, and an explicit command.
It returns the next run state and new trace events. The store writes both in one
transaction.

