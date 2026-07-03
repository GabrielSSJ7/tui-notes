//! tui-notes: a terminal notes browser over a directory of `.md`/`.txt` files
//! with fuzzy filename search, neovim editing, and sqlite-backed reminders.

pub mod app;
pub mod editor;
pub mod fuzzy;
pub mod models;
pub mod notes;
pub mod store;
pub mod ui;
