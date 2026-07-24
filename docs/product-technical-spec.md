# Alt Ctrl: Fullscreen Coding-Agent Sidecar

- **Document type:** Product and technical specification
- **Status:** Proposed
- **Target:** macOS-first MVP; Linux and Windows releases planned
- **Primary input:** DualShock 4 and DualSense controllers
- **Agent integration:** Installed Codex and Claude Code clients; no direct model API
- **Last updated:** 2026-07-24

## 1. Product summary

Alt Ctrl is a fullscreen, controller-first operations console for supervising coding agents such as Codex, Claude Code, Aider, OpenCode, and terminal-based custom agents.

Alt Ctrl integrates with locally installed agent clients and reuses their existing authentication, subscriptions, session storage, and permission systems where supported. It does not call OpenAI, Anthropic, or another provider's model inference API directly and does not require a provider API key of its own.

The product does not replace an IDE or terminal. It provides a focused sidecar for:

- switching among agent sessions and isolated worktrees;
- monitoring objectives, progress, tool calls, logs, tests, and failures;
- reviewing diffs at file and hunk level;
- approving, rejecting, interrupting, resuming, or terminating actions;
- composing short, structured follow-up instructions;
- stopping one or all agents without depending on the UI renderer.

The experience should feel like a console control surface rather than a desktop application: every primary action is reachable by controller, focus is always visible, common actions take one or two inputs, and dangerous actions require deliberate confirmation.

## 2. Goals and non-goals

### 2.1 Goals

1. Make one or more coding agents legible and controllable from a fullscreen display.
2. Make approvals, interrupts, and diff review faster than returning to each agent terminal.
3. Normalize heterogeneous agents behind a common session and event model.
4. Isolate concurrent agents in Git worktrees.
5. Keep destructive and external actions behind a sidecar-owned policy boundary.
6. Support useful operation with no keyboard after initial setup.
7. Preserve a durable audit trail of agent actions and human decisions.
8. Install as a native extension/companion for existing Codex and Claude Code clients.
9. Keep the core portable across macOS, Linux, and Windows even while the first release targets macOS.

### 2.2 Non-goals

- Replacing a full IDE, source editor, or Git client.
- Supporting unrestricted free-form coding through a controller.
- Inferring or exposing private chain-of-thought. The UI displays agent-provided output, concise status, tool activity, and explicit summaries only.
- Guaranteeing semantic integration with every agent in the MVP.
- Automatically merging concurrent work without explicit human or integration-agent approval.
- Treating controller haptics, lighting, or voice as the only path to any critical function.
- Patching, replacing, or depending on private internals of an official Codex or Claude desktop application.
- Owning model authentication, metering, or provider API billing.

## 3. Users and primary jobs

The initial user is a developer supervising one to four local coding agents while viewing a dedicated monitor or television.

Primary jobs:

- see which agents are running, blocked, waiting, failed, or complete;
- identify the next decision that requires human attention;
- inspect a requested command or patch before approving it;
- direct an agent to fix, explain, test, refactor, or revert a selected target;
- stop a runaway process immediately;
- compare agent outcomes without switching terminal windows;
- return to the IDE with a clean branch, test result, and action history.

## 4. Product principles

1. **Controller for decisions, not prose.** Prefer structured actions, recent commands, and voice over on-screen typing.
2. **Outcomes over chatter.** Prioritize objectives, changed files, tests, pending decisions, and failures above raw logs.
3. **Semantic actions over hardware codes.** UI components receive normalized actions, never controller-specific button identifiers.
4. **Safety is external to the agent.** An agent cannot grant itself permissions or alter the policy used to evaluate its actions.
5. **Interrupts survive UI failure.** Controller capture and process supervision remain available if the renderer freezes.
6. **Native client integration first.** Prefer documented local protocols, plugins, and hooks; retain a broad-compatibility PTY adapter.
7. **Isolation by default.** Each agent writes only in its assigned worktree.
8. **Visible and reversible decisions.** The user can inspect impact before approval and can trace what happened afterward.
9. **Portable core, isolated platform code.** Domain, policy, and UI state must not depend on one operating system's process, IPC, path, or controller APIs.

## 5. Experience and interaction model

### 5.1 Default layout

```text
┌──────────────────────────────────────────────────────────────┐
│ AGENTS: [API ●] [Frontend ◐] [Tests ✓] [Research …]         │
├────────────────────────────────┬─────────────────────────────┤
│                                │ Current objective           │
│ Live output / terminal         │ • Implement OAuth callback │
│                                │ • Preserve public API       │
│ Logs, tool activity, errors    │ • Add tests                 │
│                                ├─────────────────────────────┤
│                                │ Pending actions             │
│                                │ [Approve edit]              │
│                                │ [Run tests]                 │
│                                │ [Commit changes]            │
├────────────────────────────────┴─────────────────────────────┤
│ L1/R1 Agent  L2/R2 View  ✕ Confirm  ○ Back  □ Inspect       │
└──────────────────────────────────────────────────────────────┘
```

The layout contains four persistent regions:

1. session strip with state and attention indicators;
2. primary content area for output, diff, tests, or plan;
3. contextual objective and action panel;
4. input legend reflecting the current focus and available actions.

At 1920×1080, primary text must remain readable from approximately 2–3 metres. The UI must respect safe-area insets and support a large-text mode.

### 5.2 Focus and navigation

- Exactly one interactive element has focus.
- Focus is indicated by at least two cues, such as outline plus scale or color plus label.
- D-pad navigation is deterministic and never depends on pointer position.
- Left stick scrolls the focused pane; right stick performs accelerated scrolling or timeline scrubbing.
- `○` always moves toward a safe parent state and never confirms an action.
- Modal dialogs trap focus until confirmed or dismissed.
- Inactivity never auto-confirms an action.
- Repeated inputs use configurable initial delay and repeat rate.
- The active button legend updates whenever focus or mode changes.

### 5.3 Attention model

Events are ranked:

| Priority | Examples | UI behavior |
|---|---|---|
| Critical | destructive approval, global failure | persistent banner, strong haptic, no auto-dismiss |
| Action required | normal approval, agent question | badge, two short pulses |
| Failure | tests failed, adapter disconnected | error state and failure summary |
| Complete | task completed, tests passed | success state and optional short pulse |
| Informational | tool progress, file changed | timeline entry only |

Attention signals must be deduplicated and rate-limited. Selecting a notification opens the relevant agent and view.

## 6. Controller mapping

Mappings use PlayStation labels in the UI and are configurable. When the operating system reports generic gamepad labels, the input service maps them to these semantic actions.

### 6.1 Default mapping

| Control | Default action |
|---|---|
| D-pad | Move focus; navigate files, hunks, failures, and actions |
| Left stick | Scroll focused content |
| Right stick | Fast scroll, scrub timeline, or browse terminal history |
| `✕` | Confirm, execute, approve, or open |
| `○` | Back, dismiss, or cancel |
| `□` | Inspect details, diff, logs, or selected item |
| `△` | Open command palette |
| `L1` / `R1` | Previous / next agent |
| `L2` / `R2` | Previous / next view |
| `Options` | Dashboard or pause menu |
| `Share/Create` | Bookmark state or export report |
| `PS` | Emergency overview, when exposed by the OS |
| `L3` | Toggle follow-live-output |
| `R3` | Hold for voice instruction |

### 6.2 Chords and holds

| Input | Action |
|---|---|
| Hold `✕` for 1 second | Confirm a high-risk action |
| `L1 + R1` | Open global mission control |
| `L2 + R2` | Interrupt current agent |
| `Options + △` | Open raw terminal view |
| Hold `Options + ○` for 1 second | Stop all agents |
| `L1 + ✕` | Approve and continue under the current session policy |
| `R1 + □` | Review full patch |
| `L2 + □` | Show test failures |
| `R2 + □` | Show changed files |

Chord recognition must:

- use monotonic timestamps;
- require constituent inputs within a configurable 150 ms window;
- suppress the individual button actions when a chord is recognized;
- show hold progress and cancel it on early release;
- apply a 300 ms cooldown after emergency actions;
- remain active in the controller service even when the UI is unresponsive.

All mappings, thresholds, stick dead zones, repeat rates, and vibration strengths must be configurable.

## 7. Main views

### 7.1 Mission Control

Mission Control shows all known sessions as cards with:

- session name, agent kind, and connection state;
- objective;
- repository, worktree, and branch;
- running, waiting, blocked, interrupted, failed, or completed state;
- current tool call and elapsed time;
- context usage and cost estimate when the adapter supplies them;
- changed-file count and Git cleanliness;
- latest test result;
- active permission profile;
- pending approval or question count.

Selecting a card opens Agent View. Sorting defaults to action-required, failed, running, completed, then idle.

### 7.2 Agent View

Agent View is the primary supervision screen. It contains:

- live agent output with pause, follow, and searchable history;
- current objective and agent-reported plan;
- active and recent tool executions;
- pending approvals and questions;
- changed files and diff summary;
- latest test result;
- context usage, retries, failures, and elapsed time;
- actions to instruct, interrupt, resume, terminate, or open in editor.

Raw output is never discarded during a session. Rendering may virtualize older content.

### 7.3 Diff View

Diff View presents one file and one hunk at a time with:

- file status, path, language, and binary indicator;
- added, removed, and context lines with syntax-aware rendering;
- semantic summary generated from structured metadata or a separate summary step;
- file and hunk navigation;
- accept, reject, request rewrite, open in editor, and revert actions;
- warning for generated, vendored, lock, binary, or unusually large files.

If an adapter cannot apply hunk decisions natively, Alt Ctrl generates an explicit follow-up instruction, for example:

> Keep hunks 1 and 3 in `src/auth/callback.ts`, revert hunk 2, and rerun the affected tests.

The instruction is previewed before it is sent. File revert is high risk and requires a hold confirmation.

### 7.4 Test View

Test View displays:

- pass, fail, skip, and duration totals;
- active test command and progress, if known;
- a navigable list of failures;
- assertion, concise stack trace, and captured output;
- suspected changed files when derivable;
- rerun selected, rerun failed, rerun all, and ask-agent-to-fix actions.

Parsers should support common machine-readable test formats first, then bounded heuristics for terminal output. Unparsed output remains available.

### 7.5 Approval View

Each pending action shows:

- requesting agent and objective;
- action kind and exact command, path, destination, or resource;
- normalized arguments;
- risk class and the rule that assigned it;
- expected side effects;
- requested permission duration;
- approve once, approve for session, reject, and reject with instruction actions.

Approvals are immutable audit records. Editing a command creates a new instruction or action; it does not mutate the original request.

### 7.6 Task Composer

The controller-first composer builds instructions from structured fields:

```text
Agent:       current / selected agent
Action:      Fix / Explain / Test / Refactor / Revert / Continue
Target:      current file / selected hunk / failure / objective
Constraint:  minimal patch / preserve API / no dependencies / custom
Validation:  selected test / unit tests / type-check / lint / full suite
```

The generated instruction is shown as an editable preview and is sent only after confirmation. Free-form entry may use voice, keyboard, phone companion, recent instructions, or a radial phrase palette.

### 7.7 Raw Terminal View

Raw Terminal View is an escape hatch for PTY-backed adapters. It:

- renders ANSI output;
- supports scrolling and predefined key sequences;
- clearly indicates when keyboard input is required;
- does not bypass approval policy;
- warns when heuristic prompt detection is uncertain.

## 8. Agent adapter abstraction

The UI must not depend on a specific coding agent. Adapters implement a common asynchronous contract:

```rust
#[async_trait]
pub trait CodingAgentAdapter: Send + Sync {
    async fn start(&self, request: StartRequest) -> Result<AgentSession>;
    async fn send_instruction(
        &self,
        session: SessionId,
        instruction: Instruction,
    ) -> Result<()>;
    async fn approve(
        &self,
        session: SessionId,
        action: PendingActionId,
        scope: ApprovalScope,
    ) -> Result<()>;
    async fn reject(
        &self,
        session: SessionId,
        action: PendingActionId,
        reason: Option<String>,
    ) -> Result<()>;
    async fn interrupt(&self, session: SessionId) -> Result<()>;
    async fn resume(&self, session: SessionId) -> Result<()>;
    async fn terminate(&self, session: SessionId) -> Result<()>;
    async fn snapshot(&self, session: SessionId) -> Result<AgentSnapshot>;
    fn subscribe(&self, session: SessionId) -> AgentEventStream;
    fn capabilities(&self) -> AdapterCapabilities;
}
```

Normalized events:

```rust
pub enum AgentEvent {
    Output(OutputChunk),
    PlanUpdated(Plan),
    ToolStarted(ToolInvocation),
    ToolFinished(ToolResult),
    ApprovalRequested(PendingAction),
    FileChanged(FileChange),
    TestsUpdated(TestRun),
    QuestionAsked(AgentQuestion),
    ContextWarning(ContextUsage),
    StateChanged(AgentState),
    Failed(AgentFailure),
    Completed(CompletionSummary),
}
```

Each event includes `event_id`, `session_id`, monotonic sequence number, wall-clock timestamp, source, and schema version. Consumers must be idempotent and tolerate reconnect replay.

### 8.1 Capability levels

**Level 1 — PTY wrapper**

- launches the agent in a pseudo-terminal;
- reads and persists ANSI output;
- sends keystrokes and commands;
- recognizes prompts through bounded, versioned heuristics;
- tracks the managed process tree and exit status;
- marks inferred events and approvals as uncertain.

This is the broadest-compatibility fallback and the least reliable integration.

**Level 2 — Native local client adapter**

- consumes documented local JSONL/RPC streams, client plugins, lifecycle hooks, or native event interfaces;
- receives explicit tool calls, approvals, usage, state, and results;
- delegates authentication and model access to the installed coding-agent client;
- avoids terminal parsing and direct model API calls for supported operations.

**Level 3 — Sidecar protocol**

A common versioned protocol allows local or remote agents to integrate directly:

```json
{
  "schema_version": "1.0",
  "type": "approval.requested",
  "event_id": "evt_01",
  "session_id": "api-agent-01",
  "sequence": 42,
  "action": {
    "id": "act_08",
    "kind": "shell",
    "command": "cargo test --workspace",
    "risk": "low"
  }
}
```

Supported transports may include Unix domain sockets, Windows named pipes, local stdin/stdout JSONL, and authenticated WebSockets for remote operation.

### 8.2 Adapter capability negotiation

Capabilities must explicitly report support for:

- structured approvals;
- partial-diff decisions;
- plan and usage events;
- interrupt, resume, and graceful terminate;
- file and test events;
- session replay;
- remote transport.

The UI hides unsupported actions or labels a fallback behavior before execution.

### 8.3 Product-specific native adapters

#### 8.3.1 Codex

The preferred Codex integration is a local client for [`codex app-server`](https://learn.chatgpt.com/docs/app-server). App-server is the interface used to build rich Codex clients and supplies authentication, conversation history, approvals, and streamed agent events.

The Codex adapter must:

- start or connect to `codex app-server` over `stdio` or a local Unix socket where supported;
- use `stdio` as the portable fallback, including on Windows;
- complete the app-server initialization handshake and record the server version;
- generate or vendor protocol bindings for each supported Codex version;
- map thread, turn, item, approval, usage, and error notifications into normalized events;
- send instructions, approval decisions, interrupts, and session commands through the local protocol;
- reuse the user's installed Codex authentication and never request an OpenAI API key;
- detect incompatible protocol versions before starting a session;
- fall back to the PTY adapter when app-server is unavailable or incompatible.

A Codex plugin packages Alt Ctrl skills, lifecycle hooks, launcher commands, and local configuration. Plugins and hooks may enrich the integration, but the fullscreen UI remains a separate Tauri window because the supported plugin surface is not treated as a general-purpose replacement for the official desktop UI.

#### 8.3.2 Claude Code

The preferred Claude Code integration is a [Claude Code plugin](https://code.claude.com/docs/en/plugins) plus [lifecycle hooks](https://code.claude.com/docs/en/hooks) and a supervised local CLI process.

The Claude Code adapter must:

- package skills, commands, agents, and hooks under a versioned plugin;
- forward structured `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `Notification`, `Stop`, `SessionEnd`, and worktree events to the local daemon;
- use command hooks or loopback HTTP hooks with authenticated, per-session correlation IDs;
- let a `PermissionRequest` hook wait for a controller decision and fail closed if Alt Ctrl is unavailable;
- use the supervised PTY channel for live terminal output, interactive input, interrupts, and operations that hooks cannot initiate;
- reuse Claude Code's installed authentication and never request an Anthropic API key;
- expose hook and CLI-version compatibility before session start.

The Claude plugin launches or connects to the companion application; it does not attempt to inject a persistent fullscreen panel into Claude Code.

#### 8.3.3 Extension packaging

Alt Ctrl ships as three coordinated artifacts:

1. the cross-platform fullscreen companion application and local daemon;
2. an installable Codex plugin with skills, hooks, and launcher metadata;
3. an installable Claude Code plugin with skills, hooks, commands, and launcher metadata.

Extensions contain no model credentials. They locate the companion through an explicit installation manifest rather than hard-coded user paths. Extension and companion versions negotiate a small local protocol and produce an actionable upgrade error when incompatible.

## 9. System architecture

```text
DualShock / DualSense
          │
          ▼
Controller Input Service ───────────────┐
          │ semantic actions            │ emergency IPC
          ▼                             ▼
Interaction State Machine       Agent Supervisor
          │                      │ managed process trees
          ├── Fullscreen UI      ├── Session Orchestrator
          ├── Policy Client      ├── PTY Broker
          └── Event Client       └── Native Agent Adapters
                                      ├── Codex app-server
                                      ├── Claude plugin/hooks + PTY
                                      └── Generic PTY fallback
                                                │
                                                ▼
                                      Isolated Git Worktrees
```

### 9.1 Process boundaries

The production architecture uses separate processes or independently supervised services for:

1. **Fullscreen UI:** rendering, focus, view state, and user-facing notifications.
2. **Controller input service:** device discovery, mappings, chords, haptics, and emergency actions.
3. **Agent supervisor:** session lifecycle, process-tree ownership, interrupts, timeouts, and restart recovery.
4. **PTY broker:** terminal creation, I/O, ANSI stream capture, and resize.
5. **Repository observer:** Git state, file changes, diffs, and test-result ingestion.
6. **Policy engine:** permission profiles, risk classification, approvals, and audit records.
7. **Platform services:** local IPC, process control, secure storage, notifications, power events, and application launch.

The MVP may combine non-critical services in one daemon, but controller input and agent supervision must not share the renderer's failure domain.

### 9.2 Data flow

1. A native local adapter, plugin hook, or PTY fallback emits ordered normalized events.
2. The daemon validates, persists, and publishes each event.
3. State reducers derive session and view models from the event stream.
4. The UI sends semantic intents, never direct process or file operations.
5. The policy engine evaluates intents that can cause side effects.
6. The supervisor or adapter executes an authorized intent.
7. The result is appended to the event and audit logs.

### 9.3 Internal communication

- Local IPC uses Unix domain sockets on macOS and Linux, Windows named pipes on Windows, and `stdio` where an agent protocol does not expose a native platform transport.
- Messages use length-delimited, versioned JSON or MessagePack.
- Every mutating request includes a unique request ID.
- Mutating handlers are idempotent where feasible.
- Clients reconnect with the last observed sequence and request replay.
- IPC endpoints are created with current-user-only access controls.

### 9.4 Persistence

SQLite stores:

- session metadata and lifecycle;
- normalized events and output chunk references;
- approvals, rejections, and policy decisions;
- controller mappings and preferences;
- bookmarks and generated reports.

Large terminal streams may use append-only files referenced by SQLite. Secrets and raw credentials must not be persisted. Configurable redaction is applied before durable logging.

### 9.5 Platform abstraction

All operating-system behavior is exposed through narrow interfaces owned by a platform crate. Domain crates must not use conditional compilation for OS behavior.

| Capability | macOS | Linux | Windows |
|---|---|---|---|
| Local IPC | Unix domain socket | Unix domain socket | named pipe |
| Process ownership | process group | process group or cgroup when configured | Job Object |
| Graceful interrupt | `SIGINT` / `SIGTERM` | `SIGINT` / `SIGTERM` | console control event, then Job Object termination |
| PTY | POSIX PTY | POSIX PTY | ConPTY |
| Secure settings | Keychain | Secret Service/libsecret when available | Credential Manager |
| Notifications | UserNotifications | freedesktop portal/DBus | Windows App SDK |
| Packaging | signed/notarized app bundle and DMG | AppImage initially; distro packages later | signed MSIX or installer |

Additional portability rules:

- use `Path`/`PathBuf` and canonical file identities rather than string path concatenation;
- do not assume case sensitivity, executable suffixes, path separators, home-directory layout, shell, or signal semantics;
- define process termination as an ordered platform operation over an owned process tree;
- store controller mappings by stable device identity plus an editable fallback profile;
- keep OS-specific dependencies out of `sidecar-core`, protocol, reducer, and policy crates;
- run core, protocol, policy, and fixture tests on macOS, Linux, and Windows CI from the first phase.

## 10. Technology choices

### 10.1 Recommended stack

| Area | Choice | Rationale |
|---|---|---|
| Core daemon | Rust + Tokio | process safety, async I/O, and shared domain types |
| Desktop shell | Tauri | fullscreen lifecycle and Rust integration |
| Frontend | TypeScript web UI | mature text, diff, accessibility, and layout tooling |
| Terminal | xterm.js-compatible renderer | robust ANSI and large-output rendering |
| Controller | `gilrs` initially; SDL/HID backend where required | portable input with a path to platform-specific haptics and lighting |
| PTY | `portable-pty` | portable terminal process management |
| Native agent integration | Codex app-server; Claude Code plugin/hooks | structured local control using the installed clients and their authentication |
| Process supervision | platform trait over process groups and Windows Job Objects | reliable descendant cleanup on every supported OS |
| Local IPC | platform trait over Unix sockets, named pipes, and `stdio` | current-user-only communication without a network listener |
| Persistence | SQLite | durable local state with simple deployment |
| Serialization | Serde | versioned protocol and storage models |
| Repository state | Git CLI initially | matches installed Git behavior and reduces `libgit2` drift |
| File observation | `notify` | cross-platform filesystem events |
| Packaging | Tauri bundler plus OS-native signing | installable companion application on macOS, Linux, and Windows |

### 10.2 UI alternatives

- **egui/eframe:** fastest all-Rust prototype, but less capable rich diff and terminal rendering.
- **Bevy:** suitable for a highly animated game-like interface, but excessive for a text-heavy first release.
- **Custom WGPU:** maximum control with the highest implementation and accessibility cost.

The recommended initial product is a Rust orchestration daemon with a Tauri fullscreen frontend. macOS is the first delivery platform, but only the platform-services crate may depend directly on macOS APIs.

### 10.3 Platform roadmap

1. **macOS MVP:** Apple Silicon, current and previous major macOS releases, DualShock 4 and DualSense over USB or Bluetooth.
2. **Linux preview:** current Ubuntu LTS on x86-64, with Wayland and X11 smoke coverage; other distributions are best effort until packaging and controller backends are validated.
3. **Windows preview:** current supported Windows 11 on x86-64 using ConPTY, named pipes, and Job Objects.
4. **Cross-platform release:** macOS, Ubuntu LTS, and Windows 11 pass the same adapter contract, lifecycle, policy, recovery, and emergency-stop suites.
5. **Additional platforms:** other Linux distributions and architectures may be added after the primary matrix is stable.

Platform support describes Alt Ctrl itself. Availability of Codex, Claude Code, controller drivers, haptics, and light-bar features remains capability-detected per machine.

## 11. Session and worktree model

Every writable agent session receives a dedicated Git worktree:

```text
project/
project-worktrees/
├── api-agent/
├── frontend-agent/
└── test-agent/
```

```rust
pub struct AgentSession {
    pub id: SessionId,
    pub name: String,
    pub adapter: AgentKind,
    pub repository: PathBuf,
    pub worktree: PathBuf,
    pub branch: String,
    pub base_revision: String,
    pub objective: String,
    pub state: AgentState,
    pub permission_profile: PermissionProfile,
    pub started_at: DateTime<Utc>,
}
```

Rules:

- The base repository must be valid and its existing changes must be surfaced before session creation.
- Alt Ctrl never discards pre-existing changes to create a worktree.
- Worktree and branch names are deterministic, sanitized, and collision-safe.
- A session records its base revision for later comparison.
- The repository observer computes changes relative to both the base revision and working tree.
- Abandoning a session preserves its logs and branch until explicit cleanup.
- Worktree deletion is a high-risk action and is never automatic in the MVP.
- Cross-session merges require explicit approval or a designated integration session.
- Conflicts are reported as a blocked integration state, not resolved silently.

Read-only sessions may attach to an existing checkout without creating a worktree.

## 12. Approval and safety model

### 12.1 Permission profiles

| Profile | Allowed capability |
|---|---|
| Observe | Read repository and logs; no writes or process execution beyond inspection |
| Edit | Modify assigned worktree and run allow-listed safe tests; no package installation |
| Build | Edit, build, run containers, and install repository-local dependencies |
| Elevated | Explicit network, infrastructure, deployment, or external-resource access |

Profiles define ceilings, not blanket approvals. A policy rule may still require confirmation.

### 12.2 Risk classes

| Risk | Examples | Default confirmation |
|---|---|---|
| Low | read file, inspect Git state, run allow-listed unit test | auto-approve and log |
| Medium | edit files, install local package, create commit | press `✕` |
| High | delete/revert files, rewrite branch, terminate data-bearing process | hold `✕` |
| Critical | deploy, change cloud resources, publish, modify external systems | inspect, then two-step confirmation |

Risk is determined from the normalized action, resolved targets, environment, active profile, and policy rules. Ambiguous or unparsed actions fail closed.

### 12.3 Approval scopes

- **Once:** only the exact action ID.
- **Matching action for session:** same normalized operation and constrained target during the current session.
- **Session profile:** switch to a preconfigured profile; never create a new policy from agent text.

Approvals expire when the session ends, the normalized action changes, or a configured timeout elapses.

### 12.4 Global stop

Holding `Options + ○`:

1. instructs the policy engine to reject new mutating actions;
2. marks all active sessions as interrupting;
3. sends the platform's graceful interrupt to every managed process tree;
4. waits for a configurable grace period, default 3 seconds;
5. terminates surviving Unix process groups or Windows Job Objects;
6. preserves logs, diffs, worktrees, and audit history;
7. marks sessions interrupted rather than failed.

The controller service sends this command directly to the supervisor over emergency IPC. It does not depend on the renderer, frontend state, or adapter event loop.

### 12.5 Audit and redaction

Audit records include who or what initiated an action, the policy decision, confirmation method, timestamp, normalized target, outcome, and correlation IDs.

Before persistence or display, Alt Ctrl must support redaction of:

- environment variables matching secret patterns;
- known token and credential formats;
- configured file paths or output patterns.

Redaction is defense in depth and does not make it safe to emit secrets intentionally.

## 13. State machines

### 13.1 Application navigation

```text
MissionControl
├── AgentView
│   ├── OutputView
│   ├── PlanView
│   ├── DiffView
│   ├── TestView
│   └── ApprovalView
├── TaskComposer
├── CommandPalette
└── GlobalEmergency
```

Navigation is implemented as reducer-style transitions:

```rust
fn reduce(state: AppState, action: UiAction) -> Transition;

pub struct Transition {
    pub state: AppState,
    pub effects: Vec<Effect>,
}
```

Reducers are pure. Process calls, persistence, haptics, and adapter messages are effects handled outside the reducer.

### 13.2 Session lifecycle

```text
Created → Starting → Running ───────→ Completed
                         │
                         ├→ WaitingForApproval → Running
                         ├→ WaitingForAnswer ──→ Running
                         ├→ Blocked ───────────→ Running
                         ├→ Interrupting ──────→ Interrupted → Resuming → Running
                         ├→ Failing ───────────→ Failed
                         └→ Terminating ───────→ Terminated
```

Every transition is validated. Invalid or duplicate events are logged and do not corrupt the derived state.

### 13.3 Approval lifecycle

```text
Pending
├── Inspecting → Pending
├── ApprovedOnce → Executing → Succeeded | Failed
├── ApprovedForSession → Executing → Succeeded | Failed
├── Rejected
├── RejectedWithInstruction
└── Expired
```

An action cannot execute after rejection, expiration, session termination, or payload mutation.

## 14. Haptics and light bar

### 14.1 Haptic language

| Pattern | Meaning |
|---|---|
| One short pulse | action accepted |
| Two short pulses | approval or answer required |
| Soft repeating pulse | selected agent blocked |
| Strong pulse | failure |
| One long pulse | agent interrupted |
| Left/right-weighted pulse | event belongs to corresponding panel, when supported |

Repeating haptics stop after acknowledgement or a short timeout. Global intensity and all patterns can be disabled.

### 14.2 Light bar

When supported:

| State | Default indication |
|---|---|
| Idle | dim blue |
| Running | blue |
| Waiting for input | amber |
| Tests passed / completed | green |
| Tests failed | red |
| Dangerous approval pending | pulsing red |

Color is always redundant with text and iconography. The application must not rely on lighting for accessibility or safety.

## 15. Voice integration

Holding `R3` starts push-to-talk:

```text
Speech → Transcription → Structured intent → Editable preview
       → Policy evaluation → Controller confirmation → Agent instruction
```

Example input:

> Tell the API agent to keep the public interface unchanged, fix only the failing refresh test, and rerun that test.

Preview:

```text
Agent: API
Action: Fix
Target: failing refresh test
Constraints:
- preserve public interface
Validation:
- rerun selected test
```

Requirements:

- No transcript or instruction is sent without preview and confirmation.
- The preview identifies the selected agent and resolved target.
- Low-confidence transcription is visibly flagged.
- Voice is optional and unavailable states have a complete controller/keyboard fallback.
- Cloud transcription, if added, requires explicit enablement and a clear data-use notice.
- The MVP may expose the preview workflow with keyboard text while deferring speech recognition.

## 16. Functional requirements

| ID | Requirement |
|---|---|
| FR-001 | The system shall discover and reconnect a supported controller without restarting active sessions. |
| FR-002 | The system shall normalize controller input into semantic UI actions. |
| FR-003 | Every primary view and decision shall be operable without a pointer. |
| FR-004 | The system shall start, observe, instruct, interrupt, resume when supported, and terminate an agent session. |
| FR-005 | The system shall support at least one native local coding-agent adapter backed by the user's installed CLI client. |
| FR-006 | The system shall persist raw output, normalized events, session state, and audit decisions. |
| FR-007 | The system shall display repository status and changed files for each writable session. |
| FR-008 | The system shall render file diffs and navigate them by file and hunk. |
| FR-009 | The system shall display parsed test totals and failures when a supported result format is available. |
| FR-010 | The system shall classify side-effecting actions through a policy engine before execution. |
| FR-011 | High-risk actions shall require a timed hold; critical actions shall require inspection and a second confirmation. |
| FR-012 | The controller service shall interrupt the current session and stop all sessions independently of the renderer. |
| FR-013 | The system shall create an isolated Git worktree for each writable agent session unless explicitly configured read-only. |
| FR-014 | The system shall prevent an agent from changing its own permission ceiling or policy rules. |
| FR-015 | The system shall generate and preview structured follow-up instructions from selected UI context. |
| FR-016 | The system shall expose adapter capabilities and avoid presenting unsupported actions as native. |
| FR-017 | The system shall recover persisted sessions and clearly mark adapter/process connectivity after restart. |
| FR-018 | The system shall preserve logs and worktrees after interrupt, failure, or termination. |
| FR-019 | The system shall provide configurable mappings, dead zones, repeat rates, chords, and haptic intensity. |
| FR-020 | The system shall export a session report containing objective, timeline, decisions, changed files, and tests, with configured redactions applied. |
| FR-021 | The system shall provide a PTY adapter as a compatibility fallback when a native local adapter is unavailable. |
| FR-022 | The system shall reuse installed-client authentication and shall not request, store, or directly use a model-provider API key. |
| FR-023 | Codex integration shall prefer local app-server events and commands; Claude Code integration shall prefer plugin hooks plus a supervised CLI channel. |
| FR-024 | Process control, PTY, IPC, secure storage, notifications, paths, and application launch shall be implemented behind platform interfaces. |
| FR-025 | Extension and companion versions shall negotiate protocol compatibility and reject incompatible sessions with an actionable error. |
| FR-026 | The project shall provide installable extension packages for Codex and Claude Code plus an OS-native companion installer. |

## 17. Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-001 Responsiveness | Focus movement and button feedback should render within 50 ms at the 95th percentile under normal local load. |
| NFR-002 Emergency latency | Emergency IPC should reach the supervisor within 100 ms at the 95th percentile, excluding OS scheduling stalls. |
| NFR-003 Reliability | A renderer crash shall not terminate managed agents or disable controller-triggered emergency stop. |
| NFR-004 Durability | Confirmed audit records and session lifecycle events shall survive process restart once acknowledged to the UI. |
| NFR-005 Scale | The MVP shall remain responsive with 4 active sessions, 100,000 output lines per session, and 5,000 changed diff lines total. |
| NFR-006 Security | Local IPC and persisted state shall be accessible only to the current OS user by default. |
| NFR-007 Privacy | No cloud service shall receive source, logs, or voice without explicit configuration and visible disclosure. |
| NFR-008 Accessibility | Critical state shall never be communicated by color, haptics, or lighting alone; text scaling and reduced motion shall be supported. |
| NFR-009 Compatibility | The MVP shall support the current macOS release and one previous major release on Apple Silicon; the portable core shall compile and pass tests on current Ubuntu LTS and Windows 11 in CI. |
| NFR-010 Observability | Services shall emit structured logs with correlation IDs, severity, component, session ID, and redaction status. |
| NFR-011 Testability | Reducers, policy decisions, chord recognition, lifecycle transitions, and protocol decoding shall be testable without a physical controller or live agent. |
| NFR-012 Versioning | Persisted events and IPC messages shall include schema versions and support additive evolution within a major version. |
| NFR-013 Portability | OS-specific code and conditional compilation shall remain confined to platform and packaging modules. |
| NFR-014 Local integration | Starting and operating an agent through Alt Ctrl shall not require direct access to a provider's model inference API. |
| NFR-015 Graceful degradation | Unsupported haptics, lighting, notification, or native-adapter features shall degrade independently without disabling core supervision. |

## 18. MVP and delivery phases

### Phase 1 — macOS single-agent native control

- fullscreen shell and controller discovery;
- semantic controller input and navigation;
- one Codex app-server-backed session using installed-client authentication;
- generic PTY fallback and recorded fixtures;
- live ANSI output, scroll, and follow mode;
- predefined/structured instructions;
- interrupt, terminate, and renderer-independent global stop;
- Git status and changed-file list;
- session and audit persistence;
- portable core tests running on macOS, Linux, and Windows CI.

### Phase 2 — Supervision and safety

- policy engine and approval queue;
- permission profiles and risk-class confirmation;
- file/hunk diff view;
- structured test parsing;
- haptics and optional light bar;
- command history, bookmarks, and report export;
- crash recovery and reconnect replay.

### Phase 3 — Multi-agent operation

- Claude Code plugin/hooks adapter;
- multiple native and fallback adapters with concurrent sessions;
- automatic worktree creation;
- mission-control dashboard and rapid agent switching;
- integration session or explicit merge flow;
- conflict detection;
- cross-agent task delegation.

### Phase 4 — Cross-platform desktop

- Linux platform services, Ubuntu LTS packaging, and controller validation;
- Windows platform services using ConPTY, named pipes, and Job Objects;
- native extension discovery and installation on all supported operating systems;
- a common adapter contract and emergency-stop conformance suite;
- platform-specific signing, update, secure-storage, and notification behavior;
- documented capability differences for haptics, lighting, and agent clients.

### Phase 5 — Remote and companion operation

- authenticated, encrypted WebSocket transport;
- explicit machine identity and pairing;
- phone or tablet companion;
- agents on remote machines;
- remote-safe policy profiles and audit export;
- optional voice transcription service.

Each phase must retain the safety and audit guarantees of earlier phases. No phase introduces direct provider model API calls.

## 19. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| PTY prompt parsing misclassifies approval | unintended command or blocked workflow | fail closed, label inferred prompts, version heuristics, prefer structured adapters |
| Controller disconnects or OS captures buttons | lost control path | visible connection state, keyboard fallback, reconnect, supervisor-level stop command |
| Renderer freezes under terminal volume | unusable UI | process isolation, virtualized rendering, bounded UI queues, append-only output storage |
| Chord fires accidentally | unintended interrupt or stop | timing window, hold progress, cooldown, configurable mapping, audit |
| Agent escapes assigned worktree | unrelated files affected | resolved-path checks, working-directory enforcement, sandbox/permission integration where available |
| Concurrent branches conflict | integration delay or data loss | isolated worktrees, recorded base revisions, explicit merge, never auto-resolve |
| Agent output leaks credentials | sensitive data persisted or displayed | least-privilege environment, configurable redaction, restricted file permissions |
| Codex app-server or Claude hook schema changes | native sessions stop working | version negotiation, generated bindings, contract fixtures, supported-version matrix, PTY fallback |
| Installed client is missing or signed out | session cannot start | preflight diagnostics and a clear handoff to the official client login flow; never collect provider credentials |
| Platform process semantics differ | descendants survive interrupt or stop | process-tree abstraction and destructive conformance tests using Unix process groups and Windows Job Objects |
| Linux packaging and desktop environments vary | inconsistent launch, notification, or controller behavior | define Ubuntu LTS baseline, use portals where possible, publish capability diagnostics |
| Windows console behavior differs from POSIX PTYs | input, resize, or interrupt failures | ConPTY fixtures, console-control tests, and PTY fallback version gates |
| Haptics/light APIs vary by platform | inconsistent feedback | treat as optional secondary output, device capability detection |
| Voice transcription changes intent | wrong agent instruction | structured preview, confidence display, explicit confirmation |
| Git operations touch existing work | user data loss | inspect status first, never discard changes, preserve branches/worktrees |
| “Approve for session” is too broad | repeated unsafe actions | exact normalized matching, constrained targets, expiry, profile ceiling |

## 20. Acceptance criteria

### 20.1 macOS MVP

The initial implementation is acceptable when all of the following are demonstrable on a supported macOS machine:

1. A DualShock 4 or DualSense can navigate every primary MVP control from launch through session termination.
2. A user can start Codex through the local app-server adapter in an isolated worktree and see its objective, state, live output, branch, and changed files.
3. Disconnecting and reconnecting the controller restores control without losing the session.
4. `L2 + R2` interrupts the current agent, and hold `Options + ○` stops all managed process trees while the renderer is intentionally frozen.
5. Emergency stop preserves output, audit history, branch, worktree, and uncommitted changes.
6. A medium-risk action requires explicit confirmation, a high-risk action requires a visible timed hold, and a changed payload invalidates prior approval.
7. The user can open Diff View, navigate files and hunks, and generate a previewed follow-up instruction for selected hunks.
8. A supported test fixture populates pass/fail totals and lets the user open a failure and request a fix.
9. Restarting the application restores prior session history and correctly distinguishes running, disconnected, interrupted, and completed sessions.
10. Agent-provided text cannot change the active permission ceiling or policy rules.
11. Controller, reducer, policy, session-transition, protocol, and PTY-parser tests run without controller hardware or a live external agent.
12. Under the NFR-005 load fixture, focus and emergency input meet the responsiveness targets and no stored output is lost.
13. A session report can be exported with objective, event timeline, approvals, changed files, and test summary, with configured secrets redacted.
14. Failure of the UI process does not terminate the daemon, supervisor, or managed agent.
15. Alt Ctrl starts and operates the session using the installed Codex login without requesting or storing an OpenAI API key.
16. An incompatible or unavailable app-server produces a diagnostic and offers the PTY fallback without losing existing session history.
17. The portable core, protocol, reducer, policy, and fixture test suites pass in macOS, Ubuntu LTS, and Windows 11 CI.

### 20.2 Cross-platform release

The cross-platform release is acceptable when:

1. The companion installs, launches, updates, and uninstalls through documented OS-native flows on supported macOS, Ubuntu LTS, and Windows 11 versions.
2. On each platform, a supported controller can navigate the primary UI, reconnect, and trigger renderer-independent emergency stop.
3. Process-tree fixtures prove that graceful interrupt and forced termination remove all owned descendants without terminating unrelated processes.
4. Codex app-server, Claude Code hooks, and PTY adapter contract suites produce equivalent normalized lifecycle and approval events on each platform where the corresponding client is available.
5. Unsupported controller lighting, haptics, desktop notifications, or agent capabilities are reported accurately and do not disable core control.
6. Paths containing spaces and non-ASCII characters, case-insensitive filesystems, Windows drive letters, and platform-native line endings pass repository and persistence fixtures.
7. No supported integration requires Alt Ctrl to call a provider model API or possess a provider API key.

## 21. Proposed repository structure

```text
alt-ctrl/
├── Cargo.toml
├── crates/
│   ├── sidecar-core/
│   │   └── src/
│   │       ├── events.rs
│   │       ├── sessions.rs
│   │       ├── permissions.rs
│   │       ├── actions.rs
│   │       └── state.rs
│   ├── controller-input/
│   │   └── src/
│   │       ├── dualshock.rs
│   │       ├── mappings.rs
│   │       ├── chords.rs
│   │       └── haptics.rs
│   ├── agent-protocol/
│   │   └── src/
│   │       ├── messages.rs
│   │       ├── capabilities.rs
│   │       └── transport.rs
│   ├── agent-adapters/
│   │   └── src/
│   │       ├── pty.rs
│   │       ├── codex_app_server.rs
│   │       ├── claude_code_hooks.rs
│   │       └── aider.rs
│   ├── platform-services/
│   │   └── src/
│   │       ├── traits.rs
│   │       ├── macos.rs
│   │       ├── linux.rs
│   │       └── windows.rs
│   ├── repo-observer/
│   │   └── src/
│   │       ├── git.rs
│   │       ├── diff.rs
│   │       └── tests.rs
│   ├── policy-engine/
│   ├── sidecar-daemon/
│   └── controller-service/
├── extensions/
│   ├── codex/
│   │   ├── .codex-plugin/
│   │   ├── skills/
│   │   └── hooks/
│   └── claude-code/
│       ├── .claude-plugin/
│       ├── skills/
│       ├── commands/
│       └── hooks/
├── app/
│   ├── src/
│   │   ├── screens/
│   │   ├── components/
│   │   ├── state/
│   │   └── event-stream/
│   └── src-tauri/
├── protocols/
│   ├── agent-events.schema.json
│   ├── extension-ipc.schema.json
│   └── sidecar-ipc.schema.json
├── packaging/
│   ├── macos/
│   ├── linux/
│   └── windows/
├── fixtures/
│   ├── terminal-recordings/
│   ├── diffs/
│   └── test-results/
└── docs/
    └── product-technical-spec.md
```

Crate boundaries may be consolidated during Phase 1, provided domain types, renderer-independent supervision, and controller emergency handling remain separated.

## 22. Recommended initial implementation

Build a macOS-first vertical slice while keeping the core continuously buildable on Linux and Windows:

1. Define versioned, platform-neutral domain types for sessions, semantic actions, events, adapter capabilities, approvals, and audit records.
2. Define `PlatformServices` traits for IPC, process trees, PTYs, secure storage, notifications, paths, and application launch; implement the macOS backend and test doubles first.
3. Implement pure reducers and exhaustive tests for navigation, session lifecycle, approvals, holds, and chords on all three CI operating systems.
4. Build a Rust daemon that persists events in SQLite and supervises an owned process tree.
5. Implement the Codex app-server adapter over local `stdio`, including initialization, generated versioned bindings, streamed events, approval decisions, interrupts, reconnect behavior, and installed-client authentication.
6. Add the generic PTY adapter and recorded terminal fixtures as a compatibility fallback, not the primary Codex integration.
7. Build the separate controller service, including reconnect, configurable dead zones, chord recognition, and direct emergency IPC.
8. Create a Tauri fullscreen UI with Mission Control, Agent View, basic Diff View, an approval modal, and a context-sensitive controller legend.
9. Package a Codex extension that installs launcher skills and hooks without containing credentials or calling the OpenAI model API.
10. Add Git worktree creation and observation, with explicit protection for pre-existing changes.
11. Prove failure isolation by freezing and crashing the renderer while interrupting and stopping the managed process tree.
12. Add structured test-result ingestion and report export.
13. Implement the Claude Code plugin/hooks adapter and its permission-request bridge.
14. Add Linux platform services and packaging, then Windows services using ConPTY, named pipes, and Job Objects.
15. Add multi-session switching only after native adapter, safety, persistence, and portability contracts are stable.

The first release should optimize for trustworthy supervision of one local Codex session, not feature breadth. The architectural proof points are native installed-client integration without a provider API key, renderer-independent stop, deterministic controller navigation, durable normalized events, worktree isolation, policy-controlled execution, and a portable core.

## 23. Open decisions

These decisions should be resolved during the first technical spike:

1. Whether `gilrs` exposes sufficient DualShock/DualSense input, haptics, and light-bar control on each target OS or SDL/HID backends are required.
2. Which Codex app-server versions and protocol features form the initial compatibility window, and how bindings are generated and tested for each supported release.
3. Whether Claude Code hook-to-daemon communication uses command hooks over inherited `stdio`, loopback HTTP, or a platform IPC bridge.
4. Whether local IPC payloads use JSON for inspectability or MessagePack for compactness; the domain schema must remain transport-independent.
5. Whether semantic diff summaries are delegated to the installed agent, generated by an explicitly configured local model, or omitted; Alt Ctrl must not add a direct provider model API dependency.
6. Which machine-readable test formats are required for the first supported language ecosystem.
7. Which sandbox primitives complement policy checks on macOS, Linux, and Windows for enforcing worktree and network boundaries.
8. Which Linux distribution/package formats and Windows installer technology provide the smallest maintainable initial support matrix.
9. Which signing, notarization, and automatic-update system is used for each supported platform.
