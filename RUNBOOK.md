# RUNBOOK — tui-notes (LLM-first)

Operational contract for an AI agent maintaining this repo. Read this before
editing. It states what the project is, how to build/test/run it, the
invariants you must not break, and step-by-step recipes for common changes.
If reality contradicts this file, fix the code OR fix this file — never leave
them disagreeing.

## 1. What this is

A single-binary Rust TUI that browses a directory of `.md`/`.txt` notes,
opens them in **neovim**, and shows sqlite-backed **reminders** you can add
and dismiss. Invoked from a terminal; wired to Hyprland `SUPER+ALT+N`.

- Notes root: `$TUI_NOTES_DIR` or `~/.local/tui-notes`.
- Reminder DB: `<notes_root>/reminders.db` (sqlite, created on first run).
- No config file. No network. No background process.

## 2. Commands (single source of truth)

| Goal        | Command                                             |
|-------------|-----------------------------------------------------|
| Build       | `cargo build`                                       |
| Release     | `cargo build --release`                             |
| Test        | `cargo test`                                        |
| Lint        | `cargo clippy --all-targets -- -D warnings`         |
| Format      | `cargo fmt`                                          |
| Run (dev)   | `TUI_NOTES_DIR=/tmp/notes cargo run`                |
| Install     | `./install.sh`                                       |

`cargo test` is THE test command referenced by the code-style contract.
Every new function gets a test; every bug fix gets a regression test.

## 3. Module map (one responsibility each, files < 500 lines)

```
src/
  main.rs      binary entry: resolve notes dir, init/restore terminal
  lib.rs       crate root: re-exports modules for tests + binary
  app.rs       App state machine, event dispatch, data refresh  (largest file)
  models.rs    Reminder type + is_overdue
  store.rs     ReminderStore — rusqlite CRUD (add/active/dismiss)
  notes.rs     NoteTree — filesystem tree, expand/collapse, all_files
  fuzzy.rs     nucleo-matcher wrapper: filter(query, labels) -> indices
  editor.rs    edit_in_neovim: suspend TUI, spawn nvim, restore
  ui/
    mod.rs        layout + render() entry + focus_style
    tree.rs       left panel: tree rows OR search hits
    reminders.rs  right-top panel: reminders, OVERDUE styling
    preview.rs    right-bottom panel: raw text of selected note
    search.rs     top-left: search input line
    footer.rs     bottom: contextual keybind hints + status
    add_popup.rs  centered popup for two-step reminder entry
```

## 4. Data flow

1. `main` resolves the notes dir, creates it, builds `App::new`.
2. `App::new` opens `ReminderStore`, builds `NoteTree`, calls `reload`.
3. `run` loops: `terminal.draw(ui::render)` then `handle_event`.
4. Keys route by `Mode` (Normal / Search / AddReminder) in `app.rs`.
5. Editing suspends the TUI (`editor::edit_in_neovim`), then `reload` +
   `terminal.clear()` to repaint.

## 5. Invariants — do not break

- **Time is injected.** `store.rs` takes `now: i64` and dates as params; it
  never reads the clock. Keep it pure so tests stay deterministic. Only
  `app.rs`/`ui` may call `chrono::Local::now()`.
- **Only `.md` and `.txt`** are notes. Single gate: `notes::is_note_file`.
  Change extensions there and nowhere else.
- **Dismiss is soft.** `dismiss` sets `dismissed_at`; rows are never deleted.
  `active()` filters `dismissed_at IS NULL`. Preserve the audit trail.
- **Editor is neovim**, launched as `nvim <path>`. After it exits you MUST
  `terminal.clear()` and `reload()` — nvim leaves the screen dirty.
- **Search empty ⇒ tree view.** `is_searching()` is the switch; `list_len`
  and `selected_file` branch on it. Keep both branches in sync.
- Functions 4–20 lines, max 2 indent levels, early returns. Exceptions carry
  the offending value (see `parse_due`).

## 6. Common tasks (recipes)

### Add a reminder field (e.g. priority)
1. `models.rs`: add field to `Reminder` (+ test).
2. `store.rs`: `ALTER`/recreate schema in `migrate`, extend `add`,
   `map_reminder`, and the `SELECT`. Add a store test.
3. `app.rs`: extend the AddReminder stage flow if user-entered.
4. `ui/reminders.rs`: render it.

### Add a keybind
1. `app.rs`: add the arm in the relevant `on_*_key`.
2. `ui/footer.rs`: document it in the hint string for that mode.

### Support another extension
1. `notes.rs::is_note_file`: add the arm. Its unit test covers it.

### Change notes root default
1. `main.rs::resolve_notes_dir`. Keep `$TUI_NOTES_DIR` override first.

## 7. Verify before commit

```
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

Then exercise the TUI against a scratch dir:

```
mkdir -p /tmp/notes/projects && echo '# hi' > /tmp/notes/projects/a.md
TUI_NOTES_DIR=/tmp/notes cargo run
# check: tree expand (Enter on dir), / fuzzy filter, a → add reminder,
#        d dismiss, e opens nvim and returns cleanly, q quits.
```

## 8. Gotchas

- `ratatui::init()` installs a panic hook + enters the alternate screen;
  `ratatui::restore()` in `main` undoes it. Editing toggles the alternate
  screen manually in `editor.rs` — keep those two in sync if you change one.
- `rusqlite` uses the `bundled` feature: no system sqlite needed, but the
  first build compiles sqlite from source (slow once).
- Empty query in `fuzzy::filter` returns `[]` by contract — the tree view is
  the app's responsibility, not the matcher's.
