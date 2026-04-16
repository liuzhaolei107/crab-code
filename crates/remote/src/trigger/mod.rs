//! `RemoteTrigger` backend — persistent triggers and cron-scheduled wakeups.

pub mod api;
#[cfg(feature = "schedule")]
pub mod schedule;
