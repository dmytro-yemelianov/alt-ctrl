export type SessionState =
  | "running"
  | "waiting_approval"
  | "blocked"
  | "interrupted"
  | "failed"
  | "completed";

export type ConnectionState = "connected" | "reconnecting" | "disconnected";
export type Tone = "plain" | "muted" | "accent" | "warning" | "danger";
export type TestTone = "success" | "warning" | "danger";
export type PlanState = "done" | "active" | "queued";
export type AgentView = "output" | "plan" | "diff" | "tests" | "approvals";

export interface OutputLine {
  readonly tone: Tone;
  readonly text: string;
}

export interface PlanStep {
  readonly label: string;
  readonly state: PlanState;
}

export interface Approval {
  readonly id: string;
  readonly action: string;
  readonly command: string;
  readonly risk: "Medium" | "High" | "Critical";
  readonly rule: string;
  readonly sideEffect: string;
}

export interface Session {
  readonly id: string;
  readonly name: string;
  readonly agent: string;
  readonly state: SessionState;
  readonly connection: ConnectionState;
  readonly objective: string;
  readonly repository: string;
  readonly worktree: string;
  readonly branch: string;
  readonly elapsed: string;
  readonly currentTool: string;
  readonly contextPercent: number;
  readonly changedFiles: number;
  readonly gitState: string;
  readonly testSummary: string;
  readonly testTone: TestTone;
  readonly permission: "Observe" | "Edit" | "Build" | "Elevated";
  readonly attentionCount: number;
  readonly output: readonly OutputLine[];
  readonly plan: readonly PlanStep[];
  readonly approval?: Approval;
}

export interface DashboardState {
  readonly screen: "mission" | "agent" | "emergency";
  readonly sessions: readonly Session[];
  readonly focusIndex: number;
  readonly activeSessionId: string | null;
  readonly activeView: AgentView;
  readonly approvalSessionId: string | null;
  readonly controllerConnected: boolean;
  readonly notice: string;
  readonly auditCount: number;
}

export type Direction = "up" | "down" | "left" | "right";

export type DashboardAction =
  | { readonly type: "move_focus"; readonly direction: Direction }
  | { readonly type: "confirm" }
  | { readonly type: "back" }
  | { readonly type: "inspect" }
  | { readonly type: "open_session"; readonly sessionId: string }
  | { readonly type: "inspect_session"; readonly sessionId: string }
  | { readonly type: "previous_agent" }
  | { readonly type: "next_agent" }
  | { readonly type: "previous_view" }
  | { readonly type: "next_view" }
  | { readonly type: "set_view"; readonly view: AgentView }
  | { readonly type: "open_dashboard" }
  | { readonly type: "interrupt" }
  | { readonly type: "global_stop" }
  | { readonly type: "approve_once" }
  | { readonly type: "reject" }
  | { readonly type: "set_controller"; readonly connected: boolean }
  | { readonly type: "command_palette" }
  | { readonly type: "bookmark" };

export interface DashboardEffect {
  readonly type: "audit";
  readonly message: string;
}

export interface DashboardTransition {
  readonly state: DashboardState;
  readonly effects: readonly DashboardEffect[];
}

export const AGENT_VIEWS: readonly AgentView[] = [
  "output",
  "plan",
  "diff",
  "tests",
  "approvals",
];

const STATE_PRIORITY: Readonly<Record<SessionState, number>> = {
  waiting_approval: 0,
  blocked: 0,
  failed: 1,
  running: 2,
  completed: 3,
  interrupted: 4,
};

export function orderSessions(
  sessions: readonly Session[],
): readonly Session[] {
  return [...sessions].sort((left, right) => {
    const priority = STATE_PRIORITY[left.state] - STATE_PRIORITY[right.state];
    return priority === 0 ? left.name.localeCompare(right.name) : priority;
  });
}

export function initialDashboardState(
  sessions: readonly Session[],
): DashboardState {
  return {
    screen: "mission",
    sessions: orderSessions(sessions),
    focusIndex: 0,
    activeSessionId: null,
    activeView: "output",
    approvalSessionId: null,
    controllerConnected: false,
    notice: "Fixture event stream ready",
    auditCount: 148,
  };
}

export function moveGridFocus(
  current: number,
  direction: Direction,
  itemCount: number,
  columns = 2,
): number {
  if (itemCount === 0) {
    return 0;
  }

  switch (direction) {
    case "left":
      return current % columns > 0 ? current - 1 : current;
    case "right":
      return current % columns < columns - 1 && current + 1 < itemCount
        ? current + 1
        : current;
    case "up":
      return current >= columns ? current - columns : current;
    case "down":
      return current + columns < itemCount ? current + columns : current;
  }
}

function activeSession(state: DashboardState): Session | undefined {
  if (state.screen === "mission") {
    return state.sessions[state.focusIndex];
  }
  return state.sessions.find((session) => session.id === state.activeSessionId);
}

function sessionIndex(state: DashboardState, sessionId: string): number {
  return Math.max(
    0,
    state.sessions.findIndex((session) => session.id === sessionId),
  );
}

function openSession(
  state: DashboardState,
  sessionId: string,
  view: AgentView,
): DashboardState {
  return {
    ...state,
    screen: "agent",
    activeSessionId: sessionId,
    activeView: view,
    approvalSessionId: null,
    focusIndex: sessionIndex(state, sessionId),
    notice: `Opened ${state.sessions.find((session) => session.id === sessionId)?.name ?? "session"}`,
  };
}

function cycleAgent(state: DashboardState, amount: -1 | 1): DashboardState {
  const current = state.activeSessionId
    ? sessionIndex(state, state.activeSessionId)
    : state.focusIndex;
  const next =
    (current + amount + state.sessions.length) % state.sessions.length;
  const session = state.sessions[next];
  if (session === undefined) {
    return state;
  }
  return {
    ...state,
    activeSessionId:
      state.screen === "agent" ? session.id : state.activeSessionId,
    focusIndex: next,
    notice: `Selected ${session.name}`,
  };
}

function cycleView(state: DashboardState, amount: -1 | 1): DashboardState {
  if (state.screen !== "agent") {
    return state;
  }
  const current = AGENT_VIEWS.indexOf(state.activeView);
  const next = (current + amount + AGENT_VIEWS.length) % AGENT_VIEWS.length;
  return { ...state, activeView: AGENT_VIEWS[next] ?? "output" };
}

function updateSession(
  state: DashboardState,
  sessionId: string,
  update: (session: Session) => Session,
): readonly Session[] {
  return state.sessions.map((session) =>
    session.id === sessionId ? update(session) : session,
  );
}

export function reduceDashboard(
  state: DashboardState,
  action: DashboardAction,
): DashboardTransition {
  const noEffects: readonly DashboardEffect[] = [];

  switch (action.type) {
    case "move_focus":
      if (state.approvalSessionId !== null || state.screen !== "mission") {
        return { state, effects: noEffects };
      }
      return {
        state: {
          ...state,
          focusIndex: moveGridFocus(
            state.focusIndex,
            action.direction,
            state.sessions.length,
          ),
        },
        effects: noEffects,
      };
    case "open_session":
      return {
        state: openSession(state, action.sessionId, "output"),
        effects: noEffects,
      };
    case "inspect_session": {
      const session = state.sessions.find(
        (candidate) => candidate.id === action.sessionId,
      );
      if (session?.approval !== undefined) {
        return {
          state: {
            ...state,
            approvalSessionId: session.id,
            focusIndex: sessionIndex(state, session.id),
            notice: `Inspecting ${session.approval.action}`,
          },
          effects: noEffects,
        };
      }
      return {
        state: openSession(state, action.sessionId, "diff"),
        effects: noEffects,
      };
    }
    case "confirm": {
      if (state.approvalSessionId !== null) {
        return reduceDashboard(state, { type: "approve_once" });
      }
      if (state.screen !== "mission") {
        return { state, effects: noEffects };
      }
      const session = activeSession(state);
      return session === undefined
        ? { state, effects: noEffects }
        : {
            state: openSession(state, session.id, "output"),
            effects: noEffects,
          };
    }
    case "inspect": {
      const session = activeSession(state);
      return session === undefined
        ? { state, effects: noEffects }
        : reduceDashboard(state, {
            type: "inspect_session",
            sessionId: session.id,
          });
    }
    case "back":
      if (state.approvalSessionId !== null) {
        return {
          state: {
            ...state,
            approvalSessionId: null,
            notice: "Approval left pending",
          },
          effects: noEffects,
        };
      }
      return {
        state: {
          ...state,
          screen: "mission",
          activeSessionId: null,
          approvalSessionId: null,
          notice: "Mission Control",
        },
        effects: noEffects,
      };
    case "previous_agent":
      return { state: cycleAgent(state, -1), effects: noEffects };
    case "next_agent":
      return { state: cycleAgent(state, 1), effects: noEffects };
    case "previous_view":
      return { state: cycleView(state, -1), effects: noEffects };
    case "next_view":
      return { state: cycleView(state, 1), effects: noEffects };
    case "set_view":
      return {
        state:
          state.screen === "agent"
            ? { ...state, activeView: action.view }
            : state,
        effects: noEffects,
      };
    case "open_dashboard":
      return {
        state: {
          ...state,
          screen: "mission",
          activeSessionId: null,
          approvalSessionId: null,
          notice: "Mission Control",
        },
        effects: noEffects,
      };
    case "interrupt": {
      const session = activeSession(state);
      if (
        session === undefined ||
        ["completed", "interrupted"].includes(session.state)
      ) {
        return { state, effects: noEffects };
      }
      return {
        state: {
          ...state,
          sessions: updateSession(state, session.id, (current) => ({
            ...current,
            state: "interrupted",
            currentTool: "Interrupted by operator",
          })),
          notice: `${session.name} interrupted · fixture only`,
          auditCount: state.auditCount + 1,
        },
        effects: [
          { type: "audit", message: `Interrupt requested for ${session.name}` },
        ],
      };
    }
    case "global_stop": {
      const sessions = state.sessions.map((session): Session => {
        if (session.state === "completed" || session.state === "interrupted") {
          return session;
        }
        return {
          ...session,
          state: "interrupted",
          currentTool: "Stopped by global emergency action",
        };
      });
      return {
        state: {
          ...state,
          screen: "emergency",
          sessions,
          approvalSessionId: null,
          notice: "Global stop completed · fixture processes only",
          auditCount: state.auditCount + 1,
        },
        effects: [{ type: "audit", message: "Global stop requested" }],
      };
    }
    case "approve_once": {
      const sessionId = state.approvalSessionId;
      const session = state.sessions.find(
        (candidate) => candidate.id === sessionId,
      );
      if (session === undefined || session.approval === undefined) {
        return { state, effects: noEffects };
      }
      return {
        state: {
          ...state,
          sessions: updateSession(state, session.id, (current) => {
            const { approval: _approval, ...withoutApproval } = current;
            return {
              ...withoutApproval,
              state: "running",
              attentionCount: 0,
              currentTool: current.approval?.command ?? current.currentTool,
            };
          }),
          approvalSessionId: null,
          notice: `${session.name} approved once · fixture only`,
          auditCount: state.auditCount + 1,
        },
        effects: [
          { type: "audit", message: `Approved ${session.approval.id} once` },
        ],
      };
    }
    case "reject": {
      const sessionId = state.approvalSessionId;
      const session = state.sessions.find(
        (candidate) => candidate.id === sessionId,
      );
      if (session === undefined || session.approval === undefined) {
        return { state, effects: noEffects };
      }
      return {
        state: {
          ...state,
          sessions: updateSession(state, session.id, (current) => {
            const { approval: _approval, ...withoutApproval } = current;
            return {
              ...withoutApproval,
              state: "blocked",
              attentionCount: 1,
              currentTool: "Action rejected by operator",
            };
          }),
          approvalSessionId: null,
          notice: `${session.name} action rejected · fixture only`,
          auditCount: state.auditCount + 1,
        },
        effects: [
          { type: "audit", message: `Rejected ${session.approval.id}` },
        ],
      };
    }
    case "set_controller":
      return {
        state: {
          ...state,
          controllerConnected: action.connected,
          notice: action.connected
            ? "Controller connected"
            : "Keyboard simulation active",
        },
        effects: noEffects,
      };
    case "command_palette":
      return {
        state: {
          ...state,
          notice: "Command palette is planned for the next slice",
        },
        effects: noEffects,
      };
    case "bookmark":
      return {
        state: {
          ...state,
          notice: "State bookmarked · fixture only",
          auditCount: state.auditCount + 1,
        },
        effects: [{ type: "audit", message: "State bookmarked" }],
      };
  }
}
