import "./style.css";

import { fixtureSessions } from "./fixtures";
import { SemanticInput, type HoldState } from "./input";
import {
  AGENT_VIEWS,
  initialDashboardState,
  reduceDashboard,
  type AgentView,
  type DashboardAction,
  type DashboardState,
  type Session,
  type SessionState,
} from "./model";

function requireAppRoot(): HTMLDivElement {
  const element = document.querySelector<HTMLDivElement>("#app");
  if (element === null) {
    throw new Error("missing #app root");
  }
  return element;
}

const app = requireAppRoot();
let state = initialDashboardState(fixtureSessions);
let holdState: HoldState = { active: false, progress: 0 };

const escapeHtml = (value: string): string =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");

const stateLabel: Readonly<Record<SessionState, string>> = {
  running: "Running",
  waiting_approval: "Approval needed",
  blocked: "Blocked",
  interrupted: "Interrupted",
  failed: "Tests failed",
  completed: "Complete",
};

const stateIcon: Readonly<Record<SessionState, string>> = {
  running: "↻",
  waiting_approval: "!",
  blocked: "Ⅱ",
  interrupted: "■",
  failed: "×",
  completed: "✓",
};

function dispatch(action: DashboardAction): void {
  const transition = reduceDashboard(state, action);
  state = transition.state;
  render();
}

function sessionCard(session: Session, index: number): string {
  const focused = state.focusIndex === index;
  const attention =
    session.attentionCount > 0
      ? `<span class="attention-count" aria-label="${session.attentionCount} pending items">${session.attentionCount}</span>`
      : "";
  return `
    <button
      type="button"
      class="session-card state-${session.state}${focused ? " is-focused" : ""}"
      data-session-id="${escapeHtml(session.id)}"
      data-focused="${String(focused)}"
      tabindex="${focused ? "0" : "-1"}"
      aria-label="${escapeHtml(session.name)}, ${stateLabel[session.state]}"
    >
      <span class="focus-tag">SELECTED</span>
      <span class="card-topline">
        <span class="agent-mark">${escapeHtml(session.agent.slice(0, 1))}</span>
        <span class="card-identity">
          <strong>${escapeHtml(session.name)}</strong>
          <small>${escapeHtml(session.agent)} · ${escapeHtml(session.connection)}</small>
        </span>
        ${attention}
        <span class="state-chip"><i>${stateIcon[session.state]}</i>${stateLabel[session.state]}</span>
      </span>

      <span class="objective">${escapeHtml(session.objective)}</span>

      <span class="tool-row">
        <span class="tool-pulse"></span>
        <span>${escapeHtml(session.currentTool)}</span>
        <time>${escapeHtml(session.elapsed)}</time>
      </span>

      <span class="meta-grid">
        <span><small>BRANCH</small><b>${escapeHtml(session.branch)}</b></span>
        <span><small>CHANGES</small><b>${session.changedFiles} files · ${escapeHtml(session.gitState)}</b></span>
        <span><small>TESTS</small><b class="text-${session.testTone}">${escapeHtml(session.testSummary)}</b></span>
        <span><small>POLICY</small><b>${escapeHtml(session.permission)}</b></span>
      </span>

      <span class="context-row">
        <small>CONTEXT</small>
        <span class="meter"><i style="width: ${session.contextPercent}%"></i></span>
        <b>${session.contextPercent}%</b>
      </span>
    </button>
  `;
}

function attentionRail(): string {
  const attentionSessions = state.sessions.filter(
    (session) => session.attentionCount > 0 || session.state === "failed",
  );
  return `
    <aside class="attention-rail" aria-label="Attention queue">
      <div class="rail-heading">
        <span>
          <small>OPERATOR QUEUE</small>
          <h2>Needs attention</h2>
        </span>
        <b>${attentionSessions.reduce((sum, session) => sum + session.attentionCount, 0)}</b>
      </div>
      <div class="attention-list">
        ${attentionSessions
          .map(
            (session) => `
              <button type="button" data-inspect-session="${escapeHtml(session.id)}" tabindex="-1">
                <i class="attention-icon state-${session.state}">${stateIcon[session.state]}</i>
                <span>
                  <strong>${escapeHtml(session.name)}</strong>
                  <small>${
                    session.approval === undefined
                      ? escapeHtml(session.testSummary)
                      : escapeHtml(session.approval.action)
                  }</small>
                </span>
                <em>□</em>
              </button>
            `,
          )
          .join("")}
      </div>
      <div class="system-card">
        <div><span class="live-dot"></span><small>DAEMON</small><b>Healthy</b></div>
        <div><span class="live-dot"></span><small>EVENT STORE</small><b>${state.auditCount} events</b></div>
        <div><span class="live-dot muted"></span><small>ADAPTERS</small><b>3 / 4 connected</b></div>
      </div>
      <div class="rail-note">
        <span>SIMULATION</span>
        Fixture sessions only. No commands or processes are executed.
      </div>
    </aside>
  `;
}

function missionControl(): string {
  const running = state.sessions.filter(
    (session) => session.state === "running",
  ).length;
  const actionRequired = state.sessions.filter(
    (session) =>
      session.state === "waiting_approval" || session.state === "blocked",
  ).length;
  return `
    <main class="mission-layout">
      <section class="mission-main" aria-labelledby="screen-title">
        <div class="screen-heading">
          <span>
            <p class="eyebrow">LOCAL OPERATIONS / 04 SESSIONS</p>
            <h1 id="screen-title">Mission Control</h1>
            <p>Supervise every agent. Intervene only when it matters.</p>
          </span>
          <div class="summary-pills" aria-label="Session summary">
            <span><i class="cyan"></i>${running} running</span>
            <span><i class="amber"></i>${actionRequired} action required</span>
            <span><i class="red"></i>1 failed</span>
          </div>
        </div>
        <div class="session-grid" aria-label="Coding agent sessions">
          ${state.sessions.map(sessionCard).join("")}
        </div>
      </section>
      ${attentionRail()}
    </main>
  `;
}

function outputPanel(session: Session): string {
  return `
    <section class="agent-output panel">
      <div class="panel-heading">
        <span><small>LIVE STREAM</small><h2>Agent output</h2></span>
        <span class="stream-state"><i></i> FOLLOWING</span>
      </div>
      <div class="terminal" role="log" aria-label="Agent output fixture">
        <div class="terminal-bar">
          <span></span><span></span><span></span>
          <b>${escapeHtml(session.worktree)}</b>
        </div>
        <div class="terminal-lines">
          ${session.output
            .map(
              (line, index) =>
                `<p class="line-${line.tone}"><small>${String(index + 1).padStart(2, "0")}</small>${escapeHtml(line.text)}</p>`,
            )
            .join("")}
          <p class="terminal-cursor"><small>›</small><i></i></p>
        </div>
      </div>
    </section>
  `;
}

function planPanel(session: Session): string {
  return `
    <section class="plan-panel panel">
      <div class="panel-heading">
        <span><small>OBJECTIVE</small><h2>Execution plan</h2></span>
        <b>${session.plan.filter((step) => step.state === "done").length}/${session.plan.length}</b>
      </div>
      <p class="plan-objective">${escapeHtml(session.objective)}</p>
      <ol class="plan-list">
        ${session.plan
          .map(
            (step) => `
              <li class="plan-${step.state}">
                <i>${step.state === "done" ? "✓" : step.state === "active" ? "↻" : "·"}</i>
                <span>${escapeHtml(step.label)}</span>
              </li>
            `,
          )
          .join("")}
      </ol>
    </section>
  `;
}

function placeholderPanel(session: Session, view: AgentView): string {
  const content: Record<
    Exclude<AgentView, "output" | "plan">,
    [string, string, string]
  > = {
    diff: [
      "CHANGED FILES",
      `${session.changedFiles} files in review`,
      "+284  −71",
    ],
    tests: ["LATEST TEST RUN", session.testSummary, session.currentTool],
    approvals: [
      "POLICY QUEUE",
      session.approval?.action ?? "No pending approvals",
      session.approval?.rule ?? `${session.permission} profile active`,
    ],
  };
  const [eyebrow, title, detail] =
    content[view as Exclude<AgentView, "output" | "plan">];
  return `
    <section class="placeholder-panel panel">
      <small>${eyebrow}</small>
      <div class="placeholder-glyph">${view === "diff" ? "±" : view === "tests" ? "✓" : "!"}</div>
      <h2>${escapeHtml(title)}</h2>
      <p>${escapeHtml(detail)}</p>
      <span>This focused view lands in the next vertical slice.</span>
    </section>
  `;
}

function agentView(): string {
  const session = state.sessions.find(
    (candidate) => candidate.id === state.activeSessionId,
  );
  if (session === undefined) {
    return missionControl();
  }
  const body =
    state.activeView === "output"
      ? `${outputPanel(session)}${planPanel(session)}`
      : state.activeView === "plan"
        ? `${planPanel(session)}${outputPanel(session)}`
        : placeholderPanel(session, state.activeView);
  return `
    <main class="agent-layout">
      <header class="agent-heading">
        <span class="agent-mark large">${escapeHtml(session.agent.slice(0, 1))}</span>
        <span class="agent-title">
          <small>${escapeHtml(session.agent)} / ${escapeHtml(session.branch)}</small>
          <h1>${escapeHtml(session.name)}</h1>
        </span>
        <span class="state-chip state-${session.state}"><i>${stateIcon[session.state]}</i>${stateLabel[session.state]}</span>
        <span class="agent-time"><small>ELAPSED</small><b>${escapeHtml(session.elapsed)}</b></span>
      </header>
      <nav class="view-tabs" aria-label="Agent views">
        ${AGENT_VIEWS.map(
          (view) => `
            <button
              type="button"
              data-view="${view}"
              class="${state.activeView === view ? "active" : ""}"
              tabindex="${state.activeView === view ? "0" : "-1"}"
            >${view}<em>${view === "approvals" && session.approval !== undefined ? "1" : ""}</em></button>
          `,
        ).join("")}
      </nav>
      <div class="agent-content ${state.activeView}-content">${body}</div>
      <aside class="agent-facts">
        <span><small>REPOSITORY</small><b>${escapeHtml(session.repository)}</b></span>
        <span><small>WORKTREE</small><b>${escapeHtml(session.worktree)}</b></span>
        <span><small>CHANGES</small><b>${session.changedFiles} files</b></span>
        <span><small>CONTEXT</small><b>${session.contextPercent}%</b></span>
        <span><small>PERMISSION</small><b>${escapeHtml(session.permission)}</b></span>
      </aside>
    </main>
  `;
}

function emergencyView(): string {
  const interrupted = state.sessions.filter(
    (session) => session.state === "interrupted",
  ).length;
  return `
    <main class="emergency-screen">
      <div class="emergency-symbol">■</div>
      <p class="eyebrow">GLOBAL SAFETY ACTION / SIMULATION</p>
      <h1>All active agents stopped</h1>
      <p>${interrupted} fixture sessions are interrupted. Worktrees, output, and audit history remain preserved.</p>
      <div class="emergency-steps">
        <span><i>✓</i>New mutations blocked</span>
        <span><i>✓</i>Interrupt dispatched</span>
        <span><i>✓</i>History preserved</span>
      </div>
      <button type="button" data-dashboard tabindex="0">Return to Mission Control <kbd>○</kbd></button>
    </main>
  `;
}

function approvalOverlay(): string {
  const session = state.sessions.find(
    (candidate) => candidate.id === state.approvalSessionId,
  );
  const approval = session?.approval;
  if (session === undefined || approval === undefined) {
    return "";
  }
  return `
    <div class="modal-scrim">
      <section
        class="approval-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="approval-title"
        tabindex="0"
      >
        <div class="modal-topline">
          <span class="risk-chip">${escapeHtml(approval.risk)} risk</span>
          <span>REQUEST ${escapeHtml(approval.id)}</span>
        </div>
        <p class="eyebrow">${escapeHtml(session.name)} / APPROVAL REQUIRED</p>
        <h1 id="approval-title">${escapeHtml(approval.action)}</h1>
        <div class="command-preview"><code>${escapeHtml(approval.command)}</code></div>
        <dl>
          <div><dt>POLICY RULE</dt><dd>${escapeHtml(approval.rule)}</dd></div>
          <div><dt>EXPECTED EFFECT</dt><dd>${escapeHtml(approval.sideEffect)}</dd></div>
          <div><dt>SCOPE</dt><dd>Exact action · approve once</dd></div>
        </dl>
        <div class="modal-actions">
          <span class="primary"><kbd>✕</kbd><b>Approve once</b></span>
          <span><kbd>R</kbd><b>Reject</b></span>
          <span><kbd>○</kbd><b>Keep pending</b></span>
        </div>
        <p class="fixture-warning">Simulation only — no command will run.</p>
      </section>
    </div>
  `;
}

function controllerLegend(): string {
  const agent = state.screen === "agent";
  return `
    <footer class="controller-legend" aria-label="Controller actions">
      <span><kbd>↑↓←→</kbd><b>${agent ? "Navigate" : "Move focus"}</b></span>
      <span><kbd>✕</kbd><b>${state.approvalSessionId === null ? "Open" : "Approve once"}</b></span>
      <span><kbd>○</kbd><b>Back</b></span>
      <span><kbd>□</kbd><b>Inspect</b></span>
      <span><kbd>${agent ? "L2 / R2" : "L1 / R1"}</kbd><b>${agent ? "Change view" : "Change agent"}</b></span>
      <span class="emergency-legend"><kbd>OPT + ○</kbd><b>Hold to stop all</b><small>Keyboard: hold G</small></span>
    </footer>
  `;
}

function shell(): string {
  const body =
    state.screen === "mission"
      ? missionControl()
      : state.screen === "agent"
        ? agentView()
        : emergencyView();
  const controllerCopy = state.controllerConnected
    ? "Controller online"
    : "Keyboard simulation";
  return `
    <div class="app-shell">
      <header class="topbar">
        <a class="brand" href="#" data-dashboard aria-label="Alt Ctrl Mission Control">
          <span>ALT</span><i></i><span>CTRL</span>
        </a>
        <div class="environment"><i></i>LOCAL MACHINE <em>/</em> ALT-CTRL</div>
        <div class="top-status">
          <span class="simulation-chip">INTERACTIVE SIMULATION</span>
          <span class="controller-status ${state.controllerConnected ? "online" : ""}">
            <i></i>${controllerCopy}
          </span>
          <time id="clock">--:--</time>
        </div>
      </header>
      ${body}
      <div class="notice" role="status" aria-live="polite">${escapeHtml(state.notice)}</div>
      ${controllerLegend()}
      ${approvalOverlay()}
      ${
        holdState.active
          ? `<div class="hold-overlay" role="alert" tabindex="0">
              <div class="hold-ring" style="--progress: ${Math.round(holdState.progress * 360)}deg">
                <span>${Math.round(holdState.progress * 100)}%</span>
              </div>
              <h2>Hold to stop all agents</h2>
              <p>Release to cancel · fixture sessions only</p>
            </div>`
          : ""
      }
    </div>
  `;
}

function bindPointerActions(): void {
  for (const card of document.querySelectorAll<HTMLElement>(
    "[data-session-id]",
  )) {
    card.addEventListener("click", () => {
      const sessionId = card.dataset["sessionId"];
      if (sessionId !== undefined) {
        dispatch({ type: "open_session", sessionId });
      }
    });
  }
  for (const item of document.querySelectorAll<HTMLElement>(
    "[data-inspect-session]",
  )) {
    item.addEventListener("click", () => {
      const sessionId = item.dataset["inspectSession"];
      if (sessionId !== undefined) {
        dispatch({ type: "inspect_session", sessionId });
      }
    });
  }
  for (const tab of document.querySelectorAll<HTMLElement>("[data-view]")) {
    tab.addEventListener("click", () => {
      const view = tab.dataset["view"] as AgentView | undefined;
      if (view !== undefined) {
        dispatch({ type: "set_view", view });
      }
    });
  }
  for (const element of document.querySelectorAll<HTMLElement>(
    "[data-dashboard]",
  )) {
    element.addEventListener("click", (event) => {
      event.preventDefault();
      dispatch({ type: "open_dashboard" });
    });
  }
}

function updateClock(): void {
  const clock = document.querySelector<HTMLTimeElement>("#clock");
  if (clock !== null) {
    const now = new Date();
    clock.dateTime = now.toISOString();
    clock.textContent = now.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }
}

function render(): void {
  app.innerHTML = shell();
  bindPointerActions();
  updateClock();
  if (holdState.active) {
    document.querySelector<HTMLElement>(".hold-overlay")?.focus({
      preventScroll: true,
    });
  } else if (state.screen === "mission" && state.approvalSessionId === null) {
    document.querySelector<HTMLElement>('[data-focused="true"]')?.focus({
      preventScroll: true,
    });
  } else if (state.approvalSessionId !== null) {
    document.querySelector<HTMLElement>(".approval-modal")?.focus({
      preventScroll: true,
    });
  } else if (state.screen === "agent") {
    document.querySelector<HTMLElement>(".view-tabs button.active")?.focus({
      preventScroll: true,
    });
  } else {
    document.querySelector<HTMLElement>(".emergency-screen button")?.focus({
      preventScroll: true,
    });
  }
}

const semanticInput = new SemanticInput(dispatch, (nextHold) => {
  const changedBucket =
    Math.round(holdState.progress * 20) !== Math.round(nextHold.progress * 20);
  const changedActivity = holdState.active !== nextHold.active;
  holdState = nextHold;
  if (changedBucket || changedActivity) {
    render();
  }
});

render();
semanticInput.start();
window.setInterval(updateClock, 10_000);
