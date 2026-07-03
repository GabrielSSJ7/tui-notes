# tui-notes

Fast terminal notes browser. A tree of your `.md`/`.txt` files, fuzzy +
full-text search, markdown-rendered preview, in-app note create/rename/delete,
neovim editing, and sqlite-backed reminders — opened instantly with a
`SUPER+ALT+N` Hyprland keybind.

```
┌─ search ───────────────┐┌─ reminders ────────┐
│ 🔍 todo_               ││ • pay rent  OVERDUE │
├─ notes ────────────────┤│ • call bob  [2026…] │
│ ▾ projects/            │├─ preview ──────────┤
│     todo.md            ││ # Todo             │
│ ▸ diary/               ││ - ship v1          │
└────────────────────────┘└────────────────────┘
 j/k move  Enter open/expand  e edit  / search  a add  d dismiss  Tab focus  q quit
```

## Install

```sh
./install.sh
```

Builds the release binary to `~/.local/bin/tui-notes`, creates the notes dir
at `~/.local/tui-notes`, and prints the Hyprland keybind to paste:

```
bind = SUPER ALT, N, exec, kitty --class tui-notes -e ~/.local/bin/tui-notes
```

## Usage

Notes live in `~/.local/tui-notes` (override with `$TUI_NOTES_DIR`). Only
`.md` and `.txt` files show up. Reminders are stored in
`~/.local/tui-notes/reminders.db`.

| Key            | Action                                       |
|----------------|----------------------------------------------|
| `j`/`k`, ↑/↓   | move selection                               |
| `Enter`/`l`    | expand/collapse dir, or open note            |
| `e`            | open selected note in neovim                 |
| `n`            | new note (in selected dir), opens neovim     |
| `R`            | rename selected note                         |
| `D`            | delete selected note (confirm with `y`)      |
| `/`            | search; `Tab` toggles filename ↔ content     |
| `a`            | add a reminder (text, then optional due)     |
| `d`            | dismiss selected reminder                    |
| `Tab`          | switch focus tree ↔ reminders                |
| `r`            | reload from disk                             |
| `q`            | quit                                         |

`.md` notes render as markdown in the preview (headings, bullets, code,
quotes); `.txt` shows raw.

### Add reminders from the shell

```sh
tui-notes remind "pay rent" --due 2026-07-10
tui-notes remind "call bob"
```

## Develop

See [RUNBOOK.md](RUNBOOK.md). Build `cargo build`, test `cargo test`,
run `TUI_NOTES_DIR=/tmp/notes cargo run`.
