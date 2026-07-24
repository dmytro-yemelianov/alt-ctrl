# Alt Ctrl

Alt Ctrl is a controller-first operations console for supervising locally
installed coding agents. It uses the clients' existing authentication and does
not call model-provider inference APIs directly.

The project is currently implementing the portable safety and interaction core
described in [the product and technical specification](docs/product-technical-spec.md).
The prioritized milestones and task backlog live in
[the implementation plan](docs/implementation-plan.md). Relevant ecosystem
patterns and license-aware adoption decisions are recorded in
[the reference project review](docs/reference-research.md).

## Interactive prototype

A fixture-backed Mission Control and Agent View prototype now lives in
[`app/`](app/README.md). It supports keyboard and standard browser gamepads and
is intentionally marked as a simulation: it does not start agents or execute
commands.

```sh
cd app
npm ci
npm run dev
```

## Development

The workspace currently contains:

- `sidecar-core`: versioned domain events, session lifecycle, and semantic UI
  actions, plus fail-closed adapter capability snapshots;
- `controller-input`: deterministic, clock-driven chord and hold recognition,
  radial deadzones, and trigger hysteresis;
- `policy-engine`: profile ceilings, risk decisions, and payload-bound approval
  grants;
- `event-store`: atomic SQLite event/checkpoint persistence with ordered replay.

Run the portable checks with:

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Frontend checks run separately:

```sh
cd app
npm test
npm run build
```

The core crates intentionally avoid operating-system APIs. Platform process,
PTY, IPC, and controller backends will be added behind narrow capability
interfaces after these contracts are stable.
