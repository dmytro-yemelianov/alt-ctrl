# Alt Ctrl implementation plan

- **Status:** Active
- **Source:** `docs/product-technical-spec.md`
- **Started:** 2026-07-24

## Review findings and proposed spec improvements

The product direction is unusually thorough for a greenfield repository. The
main improvements are sequencing and contract precision rather than a change in
scope.

1. **Move worktree isolation into the first vertical slice.** Phase 1 and the
   MVP acceptance criteria require an isolated worktree, while Phase 3 schedules
   automatic worktree creation. A single-session worktree manager belongs in
   Phase 1; Phase 3 should add multi-session naming, integration, and cleanup.
2. **Model process state and connectivity separately.** The lifecycle omits
   `Disconnected`, but restart acceptance criteria require it. Connectivity is
   orthogonal to lifecycle: a running, completed, or interrupted session can
   have an adapter that is connected, reconnecting, disconnected, or
   incompatible.
3. **Bind approvals to canonical payloads.** The spec says payload mutation
   invalidates approval but does not define canonicalization. Normalized actions
   need a stable fingerprint covering operation, resolved target, arguments,
   risk, and required capability. Execution must revalidate this fingerprint
   immediately before dispatch.
4. **Use capability-specific platform traits.** A single `PlatformServices`
   interface would become a wide portability bottleneck. Define small traits
   for process trees, PTYs, local IPC, secure storage, notifications, paths, and
   application launch, then compose them at the daemon boundary.
5. **Specify event ordering and recovery transactions.** Sequence numbers
   should be per session, with unique event IDs used for deduplication. The
   durable append and reducer checkpoint should commit atomically before the UI
   receives acknowledgement.
6. **Define chord precedence and deferral.** Buttons participating in chords
   must defer their individual action until the chord window expires. Prefer the
   most-specific matching chord, inject a monotonic clock, and keep emergency
   cooldown independent of renderer state.
7. **Pull the policy engine into the first safety slice.** Policy is listed for
   Phase 2, but Phase 1 acceptance requires confirmation by risk and payload
   invalidation. Phase 1 needs the decision and approval core; Phase 2 can add
   the full UI, persisted rule configuration, and richer classifiers.
8. **Choose explicit first-format fixtures.** The open-ended “common test
   formats” requirement should initially name Cargo/libtest JSON and JUnit XML,
   while retaining raw output for unsupported formats.
9. **Add local-protocol threat modeling.** Extension-to-daemon requests need
   current-user endpoint permissions, per-install or per-session authentication,
   request IDs, replay protection, bounded messages, and token rotation.
10. **Make performance criteria reproducible.** Check in generators for 100,000
    output lines and 5,000 diff lines, define the reference hardware class, and
    measure input-to-render and emergency-IPC latency separately.
11. **Fail closed during adapter preflight.** Capabilities should begin
    unverified, distinguish native support from named fallback behavior, and
    report missing-client, authentication, compatibility, and transport
    failures separately.
12. **Turn controller diagnostics into conformance fixtures.** A controller
    doctor should capture identity, raw reports, axis ranges, and output
    capabilities in a versioned document that is replayed through the same
    parser as live hardware.
13. **Specify dirty-work preservation before cleanup.** Worktree tests must
    cover tracked and untracked changes, ignored files, preservation conflicts,
    unsafe paths, and retry after partial cleanup.

## Architecture decisions for the initial slice

- Domain state is platform-neutral and uses serializable newtypes.
- Session lifecycle and adapter connectivity are separate state machines.
- Duplicate lifecycle transitions are idempotent; invalid transitions are
  rejected without changing state.
- Side-effecting actions carry an explicit capability and parse certainty.
- Policy profiles are ceilings. Ambiguous actions fail closed.
- Approval grants are session-bound, expiring, and tied to a SHA-256
  fingerprint of the normalized request payload.
- Controller recognition is deterministic and driven by caller-supplied
  monotonic milliseconds, so it is testable without hardware or sleeps.
- Adapter capabilities are explicit and conservative: unavailable and
  unverified operations cannot appear native, while fallback behavior must be
  named.
- Analog sticks use radial deadzone rescaling and analog triggers use separate
  press/release thresholds to prevent boundary chatter.

## Milestones and tasks

### M0 — Portable contracts and safety kernel

- [x] **ALT-001** Create the Rust workspace with portable CI targets.
- [x] **ALT-002** Define versioned event envelopes and semantic actions.
- [x] **ALT-003** Implement validated session lifecycle plus separate
  connectivity.
- [x] **ALT-004** Implement deterministic chord, hold, suppression, and cooldown
  behavior.
- [x] **ALT-005** Implement permission ceilings, risk decisions, canonical
  fingerprints, and approval revalidation.
- [x] **ALT-006** Add pure application navigation and approval reducers.
- [x] **ALT-007** Add schema-compatibility and property-based transition tests.

**Exit criteria:** portable tests pass on macOS, Ubuntu, and Windows; no
OS-specific dependency enters these crates.

### M1 — Durable single-session daemon

- [x] **ALT-101** Define event-store and snapshot interfaces and SQLite
  migrations.
- [x] **ALT-102** Atomically append events and reducer checkpoints with replay
  and deduplication.
- [ ] **ALT-103** Define narrow process-tree, local-IPC, PTY, and path traits
  with fakes.
- [ ] **ALT-104** Implement macOS process-group ownership and emergency stop.
- [ ] **ALT-105** Create and observe a protected single-session Git worktree.
- [ ] **ALT-105a** Preserve dirty tracked and non-ignored untracked work before
  teardown; retain recovery data and surface conflicts when restoration fails.
- [ ] **ALT-106** Add structured redaction before persistence and display.
- [ ] **ALT-107** Add crash/restart and owned-descendant conformance fixtures.

**Exit criteria:** a fixture process survives renderer failure, can be stopped
through daemon IPC, and replays its durable history without losing its
worktree.

### M2 — Codex vertical slice and PTY fallback

- [ ] **ALT-201** Pin the initial supported `codex app-server` protocol window
  and check in versioned fixtures.
- [ ] **ALT-202** Implement initialization, capability negotiation, and
  incompatible-version diagnostics over stdio.
- [x] **ALT-202a** Define fail-closed adapter capability and preflight snapshot
  contracts.
- [ ] **ALT-203** Map Codex thread/turn/item/approval/usage events to normalized
  envelopes.
- [ ] **ALT-204** Dispatch instructions, approvals, and interrupts with request
  IDs and payload revalidation.
- [ ] **ALT-205** Implement the supervised PTY fallback and recorded prompt
  fixtures.
- [ ] **ALT-206** Package the credential-free Codex extension and installation
  manifest.

**Exit criteria:** one installed Codex client can run in its isolated worktree
using existing authentication, with a clean fallback when app-server is
unavailable.

### M3 — Controller service and fullscreen UI

- [ ] **ALT-301** Add device discovery, reconnect, dead-zone normalization, and
  editable mapping profiles.
- [x] **ALT-301a** Implement portable radial-deadzone and trigger-hysteresis
  primitives.
- [ ] **ALT-301b** Add a versioned controller-doctor capture format whose
  reports replay through the production parser.
- [ ] **ALT-301c** Add an explicitly enabled local simulator that feeds the
  normalized input pipeline.
- [ ] **ALT-302** Run controller capture outside the renderer and connect
  emergency IPC directly to supervision.
- [ ] **ALT-303** Build deterministic focus navigation, Mission Control, and
  Agent View.
- [x] **ALT-303a** Build a fixture-backed Mission Control and Agent View
  renderer prototype with keyboard/gamepad parity, approval inspection, and a
  simulated emergency hold.
- [ ] **ALT-303b** Bind the renderer to generated/shared protocol types and the
  daemon event client inside a Tauri shell.
- [ ] **ALT-304** Add virtualized ANSI output, follow mode, and bounded UI
  queues.
- [ ] **ALT-305** Add approval and hold-confirmation UI backed by the policy
  kernel.
- [ ] **ALT-306** Add Git status, basic file/hunk Diff View, and structured
  instruction preview.
- [ ] **ALT-307** Verify the frozen-renderer emergency-stop acceptance test.

### M4 — Persistence-complete MVP

- [ ] **ALT-401** Ingest Cargo/libtest JSON and JUnit XML fixtures.
- [ ] **ALT-402** Export redacted session reports.
- [ ] **ALT-403** Add bookmarks, audit browsing, and recovery diagnostics.
- [ ] **ALT-404** Add load fixtures and latency measurement harnesses.
- [ ] **ALT-405** Complete signing, notarization, packaging, and upgrade
  diagnostics for the macOS support matrix.

### Later milestones

Claude Code hooks, multi-agent operation, Linux/Windows platform backends, and
remote operation retain the ordering in the product specification after the
single-session safety and recovery contracts pass.

## Immediate next tasks

1. verify the durable event-store transaction under crash/restart fixtures
   (`ALT-107`);
2. define narrow process, IPC, PTY, and path traits (`ALT-103`);
3. implement protected worktree creation and dirty-work preservation
   (`ALT-105`/`ALT-105a`);
4. specify the controller-doctor fixture schema (`ALT-301b`);
5. spike the installed Codex app-server protocol before committing adapter
   bindings (`ALT-201`).
