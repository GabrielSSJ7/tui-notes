//! tui-notes: a terminal notes browser over a directory of `.md`/`.txt` files
//! with fuzzy filename search, full-text search, neovim editing, in-app note
//! CRUD, and sqlite-backed reminders.

pub mod app;
pub mod cli;
pub mod content;
pub mod editor;
pub mod fs_ops;
pub mod fuzzy;
pub mod input;
pub mod md;
pub mod models;
pub mod notes;
pub mod store;
pub mod ui;
