# Reference project review

- **Reviewed:** 2026-07-24
- **Purpose:** identify proven controller and agent-orchestration patterns that
  strengthen Alt Ctrl without importing another project's architecture.

No source from these repositories is vendored. Implementations in Alt Ctrl are
written independently against its product specification. License boundaries are
recorded so future contributors know which projects are suitable for code-level
reference and which are architecture-only.

## Controller-first projects

| Project | License at review | Useful pattern | Alt Ctrl decision |
| --- | --- | --- | --- |
| [CodingMacro](https://github.com/MisterBrookT/CodingMacro) | MIT | A single agent-specific harness boundary; unsupported actions stay unmapped; interactive controller capture emits replayable fixtures | Adopt the explicit/no-guess adapter rule. Add a controller doctor whose report can be replayed as a parser fixture. |
| [OpenMicro](https://github.com/stephenleo/OpenMicro) | MIT | Controller doctor, fixture-driven device support, lifecycle compatibility matrix | Use as a second reference for diagnostic-fixture and compatibility-matrix design. |
| [VibePad](https://github.com/ignatovv/VibePad) | MIT | Radial stick deadzone, trigger hysteresis, configurable repeat timing, atomic configuration writes | Implemented portable radial-deadzone and hysteresis primitives; keep device APIs behind a future macOS backend. |
| [gamepad-cli-hub](https://github.com/PetePeter/gamepad-cli-hub) | No license found | Gamepad-driven terminal workspace UX | Product reference only; do not copy code. |
| [ClaudeGamepad](https://github.com/cch123/ClaudeGamepad) | No license found | Native macOS controller and speech UX | Product reference only; do not copy code. |
| [ClaudePad](https://github.com/talkstream/ClaudePad) | MIT | Approval-oriented controller feedback and MCP integration | Revisit when haptic language and adapter transports are implemented. |

## Orchestration projects

| Project | License at review | Useful pattern | Alt Ctrl decision |
| --- | --- | --- | --- |
| [Maestro](https://github.com/RunMaestro/Maestro) | AGPL-3.0 | Conservative capability registry, preflight status, capability-completeness checks | Architecture reference only. Added an independently designed fail-closed capability snapshot to `sidecar-core`. |
| [Agent Orchestrator](https://github.com/AgentWrapper/agent-orchestrator) | Apache-2.0 | Isolated worktrees, durable facts with derived UI state, explicit preservation and conflict tests for dirty work | Add dirty-work preservation and unsafe-path cases to worktree acceptance tests. Keep orchestration outside the first single-session slice. |
| [Awesome Agent Orchestrators](https://github.com/andyrewlee/awesome-agent-orchestrators) | No license found | Ecosystem index | Use for discovery only. Evaluate individual projects and licenses before relying on them. |

## Incorporated design rules

1. Adapter capabilities begin as `unverified`; only successful preflight may
   promote them to `native`, `fallback`, or `unsupported`.
2. A fallback capability must name the behavior the UI will present before
   execution.
3. Missing authentication is distinct from a missing or incompatible client:
   Alt Ctrl should hand the user to the installed client's login flow rather
   than silently offering a less structured path.
4. Controller diagnostic captures should include stable device identity, raw
   idle/pressed reports, axis ranges, and output-capability results. The saved
   report must run through the same parser path as live input.
5. Simulator input must enter the normalized controller pipeline and mutation
   endpoints must be disabled unless an explicit local development mode is
   active.
6. Worktree cleanup must guard path traversal, preserve tracked and untracked
   non-ignored changes, retain preservation data on conflicts, and prove that
   ignored secrets are not captured.

## Deferred candidates

The remaining names in the supplied ecosystem list are not incorporated yet.
Many were search links rather than pinned repositories, so they should be
resolved to an exact owner/repository and reviewed only when a planned Alt Ctrl
milestone needs the corresponding feature. This avoids cloning broad
orchestrators before the single-session safety path is stable.
