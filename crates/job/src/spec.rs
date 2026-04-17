//! What a job wants scheduled — the trigger side of a job definition.
//!
//! `JobSpec` describes *when* the job should fire; the *what* is a
//! [`crate::handler::JobHandler`] trait object the scheduler calls on each
//! fire. Keeping the two separate lets the same handler be reused under
//! different schedules.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How a job fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobSpec {
    /// Fire exactly once after `delay` from when [`schedule`] is called.
    ///
    /// [`schedule`]: crate::scheduler::JobScheduler::schedule
    OneShot { delay: Duration },

    /// Fire every `period`, starting `initial_delay` after scheduling.
    /// Set `initial_delay = period` for a "tick every N, first tick N
    /// from now" cadence (typical heartbeat use).
    Interval {
        period: Duration,
        initial_delay: Duration,
    },

    /// Fire at every match of a cron expression, evaluated in UTC.
    ///
    /// Expression syntax is whatever the workspace's [`croner`] crate
    /// accepts — standard 5-field `"min hour dom month dow"` plus an
    /// optional 6th `seconds` field.
    Cron { expression: String },
}

impl JobSpec {
    /// Return which [`JobKind`](crate::JobKind) this spec corresponds to.
    pub fn kind(&self) -> crate::JobKind {
        match self {
            Self::OneShot { .. } => crate::JobKind::OneShot,
            Self::Interval { .. } => crate::JobKind::Interval,
            Self::Cron { .. } => crate::JobKind::Cron,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobKind;

    #[test]
    fn kind_matches_variant() {
        let oneshot = JobSpec::OneShot {
            delay: Duration::from_secs(5),
        };
        assert_eq!(oneshot.kind(), JobKind::OneShot);

        let interval = JobSpec::Interval {
            period: Duration::from_secs(30),
            initial_delay: Duration::from_secs(30),
        };
        assert_eq!(interval.kind(), JobKind::Interval);

        let cron = JobSpec::Cron {
            expression: "0 9 * * *".into(),
        };
        assert_eq!(cron.kind(), JobKind::Cron);
    }

    #[test]
    fn serde_roundtrip() {
        let s = JobSpec::Cron {
            expression: "*/5 * * * *".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: JobSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(s.kind(), back.kind());
    }
}
