//! `crab-cron` — unified scheduling primitives for the whole workspace.
//!
//! Replaces hand-rolled `tokio::time::interval` and `sleep_until` scatter
//! across `crab-mcp` (heartbeat), `crab-agent` (proactive timers),
//! `crab-remote` (server-scheduled triggers), and user-facing cron jobs.
//! One API, one view — the TUI can render "pending jobs", the web UI can
//! show a jobs panel, and the CLI can offer `crab cron list / cancel`.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::{sync::Arc, time::Duration};
//! use crab_cron::{FnHandler, JobHandler, JobScheduler, JobSpec};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let sched = JobScheduler::new();
//! let handler: Arc<dyn JobHandler> =
//!     Arc::new(FnHandler(|_id| async move { println!("tick"); }));
//! sched
//!     .schedule(
//!         JobSpec::Interval { period: Duration::from_secs(30), initial_delay: Duration::from_secs(30) },
//!         handler,
//!     )
//!     .await?;
//! # Ok(()) }
//! ```
//!
//! ## Module layout
//!
//! - [`id`] — [`JobId`] / [`JobKind`]
//! - [`spec`] — [`JobSpec`] enum (one-shot / interval / cron)
//! - [`handler`] — [`JobHandler`] trait + [`FnHandler`] helper
//! - [`scheduler`] — [`JobScheduler`] + [`JobSnapshot`] + errors
//!
//! Persistence for cron jobs (survives process restart) lands later
//! under a `storage/` submodule once there is a real consumer.

pub mod handler;
pub mod id;
pub mod scheduler;
pub mod spec;

pub use handler::{FnHandler, JobHandler};
pub use id::{JobId, JobKind};
pub use scheduler::{JobScheduler, JobSnapshot, ScheduleError};
pub use spec::JobSpec;
