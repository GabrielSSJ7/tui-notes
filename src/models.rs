use chrono::NaiveDate;

/// A reminder persisted in sqlite. `due` is optional; a reminder with no due
/// date is never overdue and sorts after dated ones.
#[derive(Debug, Clone, PartialEq)]
pub struct Reminder {
    pub id: i64,
    pub text: String,
    pub due: Option<NaiveDate>,
    pub created_at: i64,
}

impl Reminder {
    /// True when the due date is strictly before `today`.
    ///
    /// ```
    /// # use chrono::NaiveDate;
    /// # use tui_notes::models::Reminder;
    /// let r = Reminder { id: 1, text: "pay".into(),
    ///     due: NaiveDate::from_ymd_opt(2020, 1, 1), created_at: 0 };
    /// assert!(r.is_overdue(NaiveDate::from_ymd_opt(2020, 1, 2).unwrap()));
    /// ```
    pub fn is_overdue(&self, today: NaiveDate) -> bool {
        self.due.is_some_and(|d| d < today)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn overdue_when_due_before_today() {
        let r = reminder(Some(date(2026, 1, 1)));
        assert!(r.is_overdue(date(2026, 1, 2)));
    }

    #[test]
    fn not_overdue_on_due_day() {
        let r = reminder(Some(date(2026, 1, 2)));
        assert!(!r.is_overdue(date(2026, 1, 2)));
    }

    #[test]
    fn no_due_never_overdue() {
        let r = reminder(None);
        assert!(!r.is_overdue(date(2999, 1, 1)));
    }

    fn reminder(due: Option<NaiveDate>) -> Reminder {
        Reminder {
            id: 1,
            text: "x".into(),
            due,
            created_at: 0,
        }
    }
}
