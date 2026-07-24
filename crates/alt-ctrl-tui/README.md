# Alt Ctrl TUI

The Rust terminal interface is the primary Alt Ctrl operator surface. The
current slice renders deterministic fixture sessions and never launches a real
agent, process, or command.

## Run

```sh
cargo run -p alt-ctrl-tui
```

Use `--snapshot` to print one ANSI-free frame without entering raw terminal
mode:

```sh
cargo run -q -p alt-ctrl-tui -- --snapshot
```

## Keyboard controls

| Key | Semantic action |
| --- | --- |
| Arrow keys | Move focus |
| Enter | Open, approve once, or confirm the emergency fixture |
| Escape | Back, cancel, or leave an approval pending |
| `s` | Inspect the selected session or exact pending action |
| `q` / `e` | Previous / next agent |
| `z` / `c` or Shift-Tab / Tab | Previous / next Agent View |
| `i` | Interrupt the active fixture session |
| `g` | Open the emergency overview |
| `m` | Mission Control |
| `u` | Raw terminal fixture |
| `b` | Bookmark state |
| `r` | Reject an inspected approval |
| `x` or Ctrl-C | Quit |

Production controller input remains outside the TUI failure domain. The
controller service converts hardware events into the same semantic
`sidecar-core::UiAction` values used here.
