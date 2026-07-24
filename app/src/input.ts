import type { DashboardAction, Direction } from "./model";

export interface HoldState {
  readonly active: boolean;
  readonly progress: number;
}

type Dispatch = (action: DashboardAction) => void;
type ReportHold = (state: HoldState) => void;

const BUTTON = {
  cross: 0,
  circle: 1,
  square: 2,
  triangle: 3,
  l1: 4,
  r1: 5,
  l2: 6,
  r2: 7,
  share: 8,
  options: 9,
  dpadUp: 12,
  dpadDown: 13,
  dpadLeft: 14,
  dpadRight: 15,
} as const;

const HOLD_DURATION_MS = 1_000;

export class SemanticInput {
  readonly #dispatch: Dispatch;
  readonly #reportHold: ReportHold;
  readonly #previousButtons = new Map<number, readonly boolean[]>();
  #frame: number | null = null;
  #keyboardHoldStartedAt: number | null = null;
  #gamepadHoldStartedAt: number | null = null;
  #holdCompleted = false;
  #controllerPresent = false;

  constructor(dispatch: Dispatch, reportHold: ReportHold) {
    this.#dispatch = dispatch;
    this.#reportHold = reportHold;
  }

  start(): void {
    window.addEventListener("keydown", this.#onKeyDown);
    window.addEventListener("keyup", this.#onKeyUp);
    window.addEventListener("gamepadconnected", this.#onGamepadConnection);
    window.addEventListener("gamepaddisconnected", this.#onGamepadConnection);
    this.#poll();
  }

  stop(): void {
    window.removeEventListener("keydown", this.#onKeyDown);
    window.removeEventListener("keyup", this.#onKeyUp);
    window.removeEventListener("gamepadconnected", this.#onGamepadConnection);
    window.removeEventListener(
      "gamepaddisconnected",
      this.#onGamepadConnection,
    );
    if (this.#frame !== null) {
      cancelAnimationFrame(this.#frame);
    }
  }

  readonly #onGamepadConnection = (): void => {
    this.#syncControllerStatus();
  };

  readonly #onKeyDown = (event: KeyboardEvent): void => {
    if (event.repeat && event.code !== "KeyG") {
      return;
    }

    const directionByCode: Partial<Record<string, Direction>> = {
      ArrowUp: "up",
      ArrowDown: "down",
      ArrowLeft: "left",
      ArrowRight: "right",
    };
    const direction = directionByCode[event.code];
    if (direction !== undefined) {
      event.preventDefault();
      this.#dispatch({ type: "move_focus", direction });
      return;
    }

    const actionByCode: Partial<Record<string, DashboardAction>> = {
      Enter: { type: "confirm" },
      Escape: { type: "back" },
      KeyS: { type: "inspect" },
      KeyT: { type: "command_palette" },
      KeyQ: { type: "previous_agent" },
      KeyE: { type: "next_agent" },
      KeyZ: { type: "previous_view" },
      KeyC: { type: "next_view" },
      KeyM: { type: "open_dashboard" },
      KeyI: { type: "interrupt" },
      KeyB: { type: "bookmark" },
      KeyR: { type: "reject" },
    };
    const action = actionByCode[event.code];
    if (action !== undefined) {
      event.preventDefault();
      this.#dispatch(action);
    }

    if (event.code === "KeyG" && this.#keyboardHoldStartedAt === null) {
      event.preventDefault();
      this.#beginHold(performance.now(), "keyboard");
    }
  };

  readonly #onKeyUp = (event: KeyboardEvent): void => {
    if (event.code === "KeyG") {
      this.#keyboardHoldStartedAt = null;
      this.#cancelHoldIfReleased();
    }
  };

  #poll = (): void => {
    this.#syncControllerStatus();
    const gamepad = [...(navigator.getGamepads?.() ?? [])].find(
      (candidate): candidate is Gamepad => candidate !== null,
    );
    if (gamepad !== undefined) {
      this.#readGamepad(gamepad);
    }
    this.#updateHold(performance.now());
    this.#frame = requestAnimationFrame(this.#poll);
  };

  #syncControllerStatus(): void {
    const present = [...(navigator.getGamepads?.() ?? [])].some(
      (gamepad) => gamepad !== null,
    );
    if (present !== this.#controllerPresent) {
      this.#controllerPresent = present;
      if (!present) {
        this.#previousButtons.clear();
        this.#gamepadHoldStartedAt = null;
        this.#cancelHoldIfReleased();
      }
      this.#dispatch({ type: "set_controller", connected: present });
    }
  }

  #readGamepad(gamepad: Gamepad): void {
    const current = gamepad.buttons.map((button) => button.pressed);
    const previous = this.#previousButtons.get(gamepad.index) ?? [];
    const pressed = (index: number): boolean => current[index] === true;
    const edge = (index: number): boolean =>
      current[index] === true && previous[index] !== true;

    const dashboardChord = pressed(BUTTON.l1) && pressed(BUTTON.r1);
    const interruptChord = pressed(BUTTON.l2) && pressed(BUTTON.r2);
    const emergencyChord = pressed(BUTTON.options) && pressed(BUTTON.circle);

    if (dashboardChord && (edge(BUTTON.l1) || edge(BUTTON.r1))) {
      this.#dispatch({ type: "open_dashboard" });
    }
    if (interruptChord && (edge(BUTTON.l2) || edge(BUTTON.r2))) {
      this.#dispatch({ type: "interrupt" });
    }
    if (emergencyChord && this.#gamepadHoldStartedAt === null) {
      this.#beginHold(performance.now(), "gamepad");
    } else if (!emergencyChord && this.#gamepadHoldStartedAt !== null) {
      this.#gamepadHoldStartedAt = null;
      this.#cancelHoldIfReleased();
    }

    const dpad: readonly [number, Direction][] = [
      [BUTTON.dpadUp, "up"],
      [BUTTON.dpadDown, "down"],
      [BUTTON.dpadLeft, "left"],
      [BUTTON.dpadRight, "right"],
    ];
    for (const [button, direction] of dpad) {
      if (edge(button)) {
        this.#dispatch({ type: "move_focus", direction });
      }
    }

    if (!dashboardChord) {
      if (edge(BUTTON.l1)) this.#dispatch({ type: "previous_agent" });
      if (edge(BUTTON.r1)) this.#dispatch({ type: "next_agent" });
    }
    if (!interruptChord) {
      if (edge(BUTTON.l2)) this.#dispatch({ type: "previous_view" });
      if (edge(BUTTON.r2)) this.#dispatch({ type: "next_view" });
    }
    if (!emergencyChord && edge(BUTTON.circle))
      this.#dispatch({ type: "back" });
    if (edge(BUTTON.cross)) this.#dispatch({ type: "confirm" });
    if (edge(BUTTON.square)) this.#dispatch({ type: "inspect" });
    if (edge(BUTTON.triangle)) this.#dispatch({ type: "command_palette" });
    if (edge(BUTTON.options) && !emergencyChord)
      this.#dispatch({ type: "open_dashboard" });
    if (edge(BUTTON.share)) this.#dispatch({ type: "bookmark" });

    this.#previousButtons.set(gamepad.index, current);
  }

  #beginHold(now: number, source: "keyboard" | "gamepad"): void {
    if (source === "keyboard") {
      this.#keyboardHoldStartedAt = now;
    } else {
      this.#gamepadHoldStartedAt = now;
    }
    this.#holdCompleted = false;
    this.#reportHold({ active: true, progress: 0 });
  }

  #updateHold(now: number): void {
    const startedAt = this.#keyboardHoldStartedAt ?? this.#gamepadHoldStartedAt;
    if (startedAt === null || this.#holdCompleted) {
      return;
    }
    const progress = Math.min(1, (now - startedAt) / HOLD_DURATION_MS);
    this.#reportHold({ active: true, progress });
    if (progress >= 1) {
      this.#holdCompleted = true;
      this.#reportHold({ active: false, progress: 1 });
      this.#dispatch({ type: "global_stop" });
    }
  }

  #cancelHoldIfReleased(): void {
    if (
      this.#keyboardHoldStartedAt === null &&
      this.#gamepadHoldStartedAt === null
    ) {
      this.#holdCompleted = false;
      this.#reportHold({ active: false, progress: 0 });
    }
  }
}
