# Repository Guidelines

## Project Structure & Module Organization

Tracebound is in the documentation and scaffolding phase. The root `README.md` summarizes the product, planned stack, and contribution principles. Detailed product and technical requirements live in `docs/tracebound_prd.md`. Keep new long-form design documents under `docs/`; when implementation begins, follow the planned Rust layout (`src/` for application code, `tests/` for integration tests, and colocated `#[cfg(test)]` modules for unit tests). Scenario cartridges and web assets should have clearly separated directories so content changes remain independent from engine changes.

## Build, Test, and Development Commands

There is no committed Cargo manifest or executable yet, so no build or test command currently succeeds. After the Rust scaffold lands, the intended workflow is:

- `cargo run` — build and launch the local Axum application.
- `cargo test` — run deterministic engine, grading, and integration tests.
- `cargo fmt --check` — verify standard Rust formatting.
- `cargo clippy --all-targets --all-features -- -D warnings` — catch lint issues and fail on warnings.

Until then, preview Markdown changes locally and verify links and examples manually.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation) and keep Clippy clean. Name Rust modules and functions `snake_case`, types and traits `UpperCamelCase`, and constants `SCREAMING_SNAKE_CASE`. Use descriptive, stable `kebab-case` IDs for TOML scenarios, such as `phantom-inventory`. Keep simulation truth deterministic; presentation or optional narration must not alter state or grading.

## Testing Guidelines

Add tests with every simulation, grader, persistence, or cartridge-schema change. Prefer table-driven cases that verify state transitions, forbidden actions, budget exhaustion, and replay determinism. Name integration files by behavior (for example, `tests/grading_outcomes.rs`) and test functions by expected result, such as `rejects_unapproved_critical_tool`.

## Commit & Pull Request Guidelines

History currently contains only `Initial commit`, so no formal convention exists. Use concise, imperative subjects (for example, `Add deterministic grading rules`) and keep unrelated engine and scenario changes separate. Pull requests should describe the player-visible outcome, identify relevant requirements or linked issues, list validation performed, and include screenshots for UI changes. Discuss substantial changes in an issue first; favor a working vertical slice over speculative abstraction.

## Security & Configuration

Preserve the local-first, no-required-LLM design. Do not add real shell, filesystem, repository, cloud, or production access to simulated tools. Never commit secrets, API keys, local databases, or generated run state.
