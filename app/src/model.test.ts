import { describe, expect, it } from "vitest";

import { fixtureSessions } from "./fixtures";
import {
  initialDashboardState,
  moveGridFocus,
  orderSessions,
  reduceDashboard,
} from "./model";

describe("session ordering", () => {
  it("sorts action-required, failed, running, then completed sessions", () => {
    expect(
      orderSessions(fixtureSessions).map((session) => session.state),
    ).toEqual(["waiting_approval", "failed", "running", "completed"]);
  });
});

describe("grid focus", () => {
  it("moves deterministically without wrapping between rows", () => {
    expect(moveGridFocus(0, "right", 4)).toBe(1);
    expect(moveGridFocus(1, "right", 4)).toBe(1);
    expect(moveGridFocus(1, "down", 4)).toBe(3);
    expect(moveGridFocus(3, "left", 4)).toBe(2);
    expect(moveGridFocus(2, "up", 4)).toBe(0);
  });
});

describe("dashboard reducer", () => {
  it("opens the focused session and cycles views", () => {
    const initial = initialDashboardState(fixtureSessions);
    const opened = reduceDashboard(initial, { type: "confirm" }).state;
    expect(opened.screen).toBe("agent");
    expect(opened.activeView).toBe("output");

    const nextView = reduceDashboard(opened, { type: "next_view" }).state;
    expect(nextView.activeView).toBe("plan");
    expect(reduceDashboard(nextView, { type: "back" }).state.screen).toBe(
      "mission",
    );
  });

  it("inspects and approves an exact pending action", () => {
    const initial = initialDashboardState(fixtureSessions);
    const inspecting = reduceDashboard(initial, { type: "inspect" }).state;
    expect(inspecting.approvalSessionId).toBe("api-sentinel");

    const transition = reduceDashboard(inspecting, { type: "confirm" });
    const approved = transition.state.sessions.find(
      (session) => session.id === "api-sentinel",
    );
    expect(approved?.approval).toBeUndefined();
    expect(approved?.state).toBe("running");
    expect(transition.effects).toEqual([
      { type: "audit", message: "Approved approval-17 once" },
    ]);
  });

  it("global stop interrupts active fixtures but preserves completed sessions", () => {
    const initial = initialDashboardState(fixtureSessions);
    const stopped = reduceDashboard(initial, { type: "global_stop" }).state;

    expect(stopped.screen).toBe("emergency");
    expect(
      stopped.sessions
        .filter((session) => session.id !== "docs-lens")
        .every((session) => session.state === "interrupted"),
    ).toBe(true);
    expect(
      stopped.sessions.find((session) => session.id === "docs-lens")?.state,
    ).toBe("completed");
  });
});
