//! End-to-end TUI tests: launch the real binary inside a pseudo-terminal,
//! type keystrokes, then assert on side effects (sqlite + filesystem) after
//! the app exits. Assertions never scrape the ANSI screen — that is flaky;
//! observable state is not.
//!
//! Unix only (pty). The editor is stubbed via `TUI_NOTES_EDITOR=true` so the
//! `n`/`e` flows don't launch a real editor.
#![cfg(unix)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tui_notes::store::ReminderStore;

const BIN: &str = env!("CARGO_BIN_EXE_tui-notes");

/// A running TUI in a pty. Keystrokes go in via `send`; output is drained by a
/// background thread so the app never blocks writing frames.
struct TuiSession {
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
}

impl TuiSession {
    fn launch(notes_dir: &Path) -> Self {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");

        let mut cmd = CommandBuilder::new(BIN);
        cmd.env("TUI_NOTES_DIR", notes_dir);
        cmd.env("TUI_NOTES_EDITOR", "true"); // no-op editor: returns instantly
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd).expect("spawn");
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().expect("reader");
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while reader.read(&mut buf).map(|n| n > 0).unwrap_or(false) {}
        });

        let writer = pair.master.take_writer().expect("writer");
        let session = Self {
            child,
            writer,
            _master: pair.master,
        };
        session.settle(); // let the app open the DB and paint the first frame
        session
    }

    /// Send raw bytes (keystrokes) and give the event loop time to process.
    fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).expect("write");
        self.writer.flush().expect("flush");
        self.settle();
    }

    fn settle(&self) {
        thread::sleep(Duration::from_millis(200));
    }

    /// Quit with `q` and wait for the process to exit cleanly.
    fn quit(mut self) {
        self.writer.write_all(b"q").expect("write q");
        self.writer.flush().expect("flush q");
        let status = self.child.wait().expect("wait");
        assert!(status.success(), "app exited with failure: {status:?}");
    }
}

/// A fresh, unique notes dir per test. `process::id` is stable within a run;
/// the `name` disambiguates concurrent tests.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tui-notes-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn write_note(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).expect("write note");
}

#[test]
fn add_reminder_persists_to_sqlite() {
    let dir = scratch("add-reminder");

    let mut app = TuiSession::launch(&dir);
    app.send(b"a"); // open add-reminder popup
    app.send(b"buy milk"); // reminder text
    app.send(b"\r"); // Enter -> due stage
    app.send(b"\r"); // Enter (empty due) -> save
    app.quit();

    let store = ReminderStore::open(&dir.join("reminders.db")).unwrap();
    let active = store.active().unwrap();
    assert_eq!(active.len(), 1, "expected one active reminder");
    assert_eq!(active[0].text, "buy milk");
    assert!(active[0].due.is_none());
}

#[test]
fn add_then_dismiss_leaves_no_active_reminder() {
    let dir = scratch("dismiss-reminder");

    let mut app = TuiSession::launch(&dir);
    app.send(b"a");
    app.send(b"temp reminder");
    app.send(b"\r");
    app.send(b"\r");
    app.send(b"\t"); // focus reminders panel
    app.send(b"d"); // dismiss selected reminder
    app.quit();

    let store = ReminderStore::open(&dir.join("reminders.db")).unwrap();
    assert!(
        store.active().unwrap().is_empty(),
        "reminder should be dismissed"
    );
}

#[test]
fn new_note_creates_file_on_disk() {
    let dir = scratch("new-note");

    let mut app = TuiSession::launch(&dir);
    app.send(b"n"); // new-note prompt
    app.send(b"ideas"); // filename (no ext -> .md)
    app.send(b"\r"); // create + open stub editor
    app.quit();

    assert!(dir.join("ideas.md").is_file(), "note file should exist");
}

#[test]
fn new_folder_creates_dir() {
    let dir = scratch("new-folder");

    let mut app = TuiSession::launch(&dir);
    app.send(b"N"); // new-folder prompt
    app.send(b"projects");
    app.send(b"\r");
    app.quit();

    assert!(dir.join("projects").is_dir(), "folder should exist");
}

#[test]
fn rename_note_moves_file() {
    let dir = scratch("rename-note");
    write_note(&dir, "old.md", "# old\n");

    let mut app = TuiSession::launch(&dir);
    app.send(b"R"); // rename prompt, pre-filled with "old.md"
    for _ in 0..6 {
        app.send(b"\x7f"); // Backspace over "old.md"
    }
    app.send(b"new.md");
    app.send(b"\r");
    app.quit();

    assert!(dir.join("new.md").is_file(), "renamed file should exist");
    assert!(!dir.join("old.md").exists(), "old file should be gone");
}

#[test]
fn delete_note_removes_file() {
    let dir = scratch("delete-note");
    write_note(&dir, "trash.md", "junk\n");

    let mut app = TuiSession::launch(&dir);
    app.send(b"D"); // delete -> confirm mode
    app.send(b"y"); // confirm
    app.quit();

    assert!(!dir.join("trash.md").exists(), "file should be deleted");
}
