# MVP Vertical Slice Checklist

Scope: PRD Milestones 1–4, ending with one playable **The Context Vault**
encounter. Campaign progression and the other five encounters remain later work.

## Installation and walking skeleton

- [x] Stable Rust crate and Axum executable
- [x] Localhost-only default binding and printed URL
- [x] Embedded SQLite migration
- [x] Embedded static assets and server-rendered title screen
- [x] Health endpoint and structured request logging

## Domain and deterministic engine

- [x] Typed run lifecycle, approval, loadout, trace, and outcome models
- [x] Validated state transitions
- [x] Deterministic rules loaded from cartridge data
- [x] Focus accounting
- [x] State and budget graders
- [x] Critical grader prevents passing outcome
- [x] Deterministic postmortem and safe Markdown report
- [x] Successful and failed golden engine tests

## Cartridge loading

- [x] Human-readable TOML schema with initial state, context, rules, graders
- [x] Startup validation for schema version, IDs, references, and weights
- [x] Embedded production cartridge
- [x] No arbitrary code or general scripting support

## Playable Context Vault slice

- [x] Quest briefing
- [x] Context loadout with capacity validation
- [x] Expedition trace and persisted refresh-safe run state
- [x] Scenario decision intervention
- [x] SSE trace endpoint with `Last-Event-ID` recovery
- [x] Postmortem and report export
- [x] Keyboard-accessible semantic pages and visible focus styles
- [x] Vendored htmx enhancement with a complete plain-HTML fallback
- [x] Cartridge-driven approval intervention with risk, inputs, side effects, and alternative
- [ ] Campaign progression and codex persistence (Milestone 5)

## Validation

- [x] Unit and deterministic engine tests
- [x] Cartridge validation exercised by every golden and web test
- [x] Dedicated store atomicity and restart-recovery integration tests
- [x] Web route and HTML-escaping smoke tests
- [x] Approval-route persistence and SSE reconnect tests
- [ ] Full accessibility audit in a browser
