# Alt Ctrl visual prototype

This is the first interactive renderer slice for the future Tauri desktop
application. It uses deterministic fixtures and never launches commands,
modifies repositories, or controls real agent processes.

## Run it

```sh
cd app
npm ci
npm run dev
```

Open the local URL printed by Vite, normally
`http://127.0.0.1:5173`.

## Controls

The browser Gamepad API supports the standard PlayStation-style mapping. The
keyboard mirrors the semantic controller actions:

| Controller         | Keyboard   | Action                                |
| ------------------ | ---------- | ------------------------------------- |
| D-pad              | Arrow keys | Move card focus                       |
| `✕`                | Enter      | Open or approve the focused item      |
| `○`                | Escape     | Back or leave an approval pending     |
| `□`                | `S`        | Inspect an approval, diff, or failure |
| `△`                | `T`        | Command palette placeholder           |
| `L1` / `R1`        | `Q` / `E`  | Previous or next agent                |
| `L2` / `R2`        | `Z` / `C`  | Previous or next Agent View           |
| Options            | `M`        | Mission Control                       |
| Share              | `B`        | Bookmark the current state            |
| `L2 + R2`          | `I`        | Interrupt the current fixture session |
| Hold Options + `○` | Hold `G`   | Run the simulated global-stop flow    |
| —                  | `R`        | Reject the inspected fixture approval |

The production controller service remains renderer-independent. Browser
gamepad handling exists only to make this frontend slice directly testable
before the native service and Tauri transport are wired.

## Checks

```sh
npm test
npm run format:check
npm run build
```

Reducer tests cover attention ordering, deterministic focus navigation,
approval handling, view navigation, and the emergency-stop state transition.
