use chrono::{DateTime, Duration, Months, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::project::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "todo" => Self::Todo,
            "in_progress" | "inprogress" => Self::InProgress,
            "done" => Self::Done,
            "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Todo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => Self::Medium,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    /// `None` for personal tasks not tied to a project.
    pub project_id: Option<ProjectId>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_at: Option<DateTime<Utc>>,
    /// Repeat rule; set on every instance of a recurring series.
    pub recurrence: Option<Recurrence>,
    /// Shared by all instances of a recurring series (the first task's id).
    pub series_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl RecurrenceFrequency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
            Self::Yearly => "yearly",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            "quarterly" => Some(Self::Quarterly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }
}

/// Repeat rule anchored at `start` (the first occurrence's due date).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recurrence {
    pub frequency: RecurrenceFrequency,
    /// Repeat every `interval` periods; always >= 1.
    pub interval: u32,
    pub start: DateTime<Utc>,
    /// Last moment an occurrence may fall on; `None` repeats forever.
    pub until: Option<DateTime<Utc>>,
}

impl Recurrence {
    /// Safety cap for `next_after` scans (e.g. ~27 years of daily repeats).
    const MAX_STEPS: u32 = 10_000;

    /// The `n`-th occurrence counted from `start`. Computed from the anchor,
    /// not the previous occurrence, so month-end dates do not drift.
    pub fn occurrence(&self, n: u32) -> Option<DateTime<Utc>> {
        let steps = n.checked_mul(self.interval.max(1))?;
        match self.frequency {
            RecurrenceFrequency::Daily => self.start.checked_add_signed(Duration::days(steps.into())),
            RecurrenceFrequency::Weekly => {
                self.start.checked_add_signed(Duration::weeks(steps.into()))
            }
            RecurrenceFrequency::Monthly => self.start.checked_add_months(Months::new(steps)),
            RecurrenceFrequency::Quarterly => {
                self.start.checked_add_months(Months::new(steps.checked_mul(3)?))
            }
            RecurrenceFrequency::Yearly => {
                self.start.checked_add_months(Months::new(steps.checked_mul(12)?))
            }
        }
    }

    /// First occurrence strictly after `after` that is still within `until`.
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        (0..Self::MAX_STEPS)
            .map_while(|n| self.occurrence(n))
            .take_while(|at| self.until.is_none_or(|until| *at <= until))
            .find(|at| *at > after)
    }
}

#[cfg(test)]
mod recurrence_tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 16, 59, 59).unwrap()
    }

    fn rule(frequency: RecurrenceFrequency, interval: u32, until: Option<DateTime<Utc>>) -> Recurrence {
        Recurrence { frequency, interval, start: at(2026, 1, 31), until }
    }

    #[test]
    fn quarterly_keeps_month_end_without_drift() {
        let r = rule(RecurrenceFrequency::Quarterly, 1, None);
        assert_eq!(r.next_after(at(2026, 1, 31)), Some(at(2026, 4, 30)));
        assert_eq!(r.next_after(at(2026, 4, 30)), Some(at(2026, 7, 31)));
    }

    #[test]
    fn weekly_with_interval() {
        let r = rule(RecurrenceFrequency::Weekly, 2, None);
        assert_eq!(r.next_after(at(2026, 1, 31)), Some(at(2026, 2, 14)));
    }

    #[test]
    fn yearly_stops_at_until() {
        let r = rule(RecurrenceFrequency::Yearly, 1, Some(at(2027, 6, 1)));
        assert_eq!(r.next_after(at(2026, 1, 31)), Some(at(2027, 1, 31)));
        assert_eq!(r.next_after(at(2027, 1, 31)), None);
    }
}
