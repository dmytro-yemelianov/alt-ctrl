# Alt Ctrl

Alt Ctrl is a controller-first operations console for supervising locally
installed coding agents. It uses the clients' existing authentication and does
not call model-provider inference APIs directly.

The project is currently implementing the portable safety and interaction core
described in [the product and technical specification](docs/product-technical-spec.md).
The prioritized milestones and task backlog live in
[the implementation plan](docs/implementation-plan.md).

## Development

The workspace currently contains:

- `sidecar-core`: versioned domain events, session lifecycle, and semantic UI
  actions;
- `controller-input`: deterministic, clock-driven chord and hold recognition;
- `policy-engine`: profile ceilings, risk decisions, and payload-bound approval
  grants.

Run the portable checks with:

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The core crates intentionally avoid operating-system APIs. Platform process,
PTY, IPC, and controller backends will be added behind narrow capability
interfaces after these contracts are stable.
