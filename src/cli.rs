use std::path::Path;

use anyhow::{anyhow, bail, Result};
use chrono::{Local, NaiveDate};

use crate::store::ReminderStore;

const DATE_FMT: &str = "%Y-%m-%d";

/// Handle `tui-notes remind <text...> [--due YYYY-MM-DD]` without opening the
/// TUI: parse args, insert into the same reminder DB, print confirmation.
pub fn run_remind(notes_dir: &Path, args: &[String]) -> Result<()> {
    let (text, due) = parse_remind_args(args)?;
    let store = ReminderStore::open(&notes_dir.join("reminders.db"))?;
    store.add(&text, due, Local::now().timestamp())?;
    match due {
        Some(date) => println!("reminder added: {text} (due {date})"),
        None => println!("reminder added: {text}"),
    }
    Ok(())
}

/// Collect free words into the reminder text; `--due VALUE` sets the due date.
fn parse_remind_args(args: &[String]) -> Result<(String, Option<NaiveDate>)> {
    let mut words = Vec::new();
    let mut due = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--due" {
            let value = iter
                .next()
                .ok_or_else(|| anyhow!("--due needs a YYYY-MM-DD value"))?;
            due = Some(parse_due_arg(value)?);
        } else {
            words.push(arg.as_str());
        }
    }
    if words.is_empty() {
        bail!("reminder text is empty; usage: tui-notes remind <text> [--due YYYY-MM-DD]");
    }
    Ok((words.join(" "), due))
}

fn parse_due_arg(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, DATE_FMT)
        .map_err(|_| anyhow!("invalid --due '{value}', expected YYYY-MM-DD"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn joins_words_no_due() {
        let (text, due) = parse_remind_args(&args(&["buy", "milk"])).unwrap();
        assert_eq!(text, "buy milk");
        assert!(due.is_none());
    }

    #[test]
    fn parses_due_flag() {
        let (text, due) = parse_remind_args(&args(&["pay", "--due", "2026-07-10"])).unwrap();
        assert_eq!(text, "pay");
        assert_eq!(due, NaiveDate::from_ymd_opt(2026, 7, 10));
    }

    #[test]
    fn empty_text_errors() {
        assert!(parse_remind_args(&args(&["--due", "2026-07-10"])).is_err());
    }

    #[test]
    fn invalid_due_errors() {
        assert!(parse_remind_args(&args(&["x", "--due", "nope"])).is_err());
    }
}
