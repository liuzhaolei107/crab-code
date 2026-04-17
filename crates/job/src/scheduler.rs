//! `JobScheduler` — schedule / cancel / list jobs.
//!
//! Implementation strategy: one tokio task per scheduled job.
//!
//! - `OneShot`: a task that sleeps for the delay, fires once, and exits.
//! - `Interval`: a task with a `tokio::time::interval` loop; tick → fire.
//! - `Cron`: a task that on each iteration computes "next fire instant"
//!   from the cron expression via [`croner`] and sleeps until then.
//!
//! At crab's scale (tens of jobs, not thousands) the task-per-job cost is
//! negligible and gives clean fault isolation — one job panicking does
//! not touch the others. Cancellation is async-native via
//! [`tokio_util::sync::CancellationToken`], and the handler future is
//! dropped at the next `.await` boundary after cancel.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::handler::JobHandler;
use crate::id::{JobId, JobKind};
use crate::spec::JobSpec;

/// Errors raised when scheduling a job.
#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("invalid cron expression {expression:?}: {source}")]
    InvalidCron {
        expression: String,
        #[source]
        source: croner::errors::CronError,
    },
    #[error("no next fire time for cron expression {0:?}")]
    CronWithoutNextFire(String),
}

/// Lightweight snapshot of a scheduled job — what the TUI / web jobs
/// panel / CLI `crab jobs list` displays.
#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub id: JobId,
    pub kind: JobKind,
}

/// The scheduler.
///
/// Cheap to clone (internally `Arc`ed state), so the same scheduler can
/// be handed to multiple subsystems — one for mcp heartbeat, one for
/// agent proactive timers, etc. They all share the same list and one
/// `crab jobs list` shows everyone's registrations.
#[derive(Clone, Default)]
pub struct JobScheduler {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    jobs: Mutex<HashMap<JobId, Entry>>,
    next_id: std::sync::atomic::AtomicU64,
}

struct Entry {
    kind: JobKind,
    cancel: tokio_util::sync::CancellationToken,
    handle: JoinHandle<()>,
}

impl JobScheduler {
    /// Build a fresh scheduler. Does not start any background task of
    /// its own — each scheduled job carries its own tokio task.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule a new job. Returns its freshly allocated [`JobId`].
    ///
    /// The job's underlying task is spawned immediately; the scheduler
    /// does not expose a separate "start" step. For `Cron` variants this
    /// returns an error if the expression does not parse or never fires.
    pub async fn schedule(
        &self,
        spec: JobSpec,
        handler: Arc<dyn JobHandler>,
    ) -> Result<JobId, ScheduleError> {
        let kind = spec.kind();
        let id = self.allocate_id(kind);
        let cancel = tokio_util::sync::CancellationToken::new();

        let handle = match spec {
            JobSpec::OneShot { delay } => spawn_oneshot(id.clone(), delay, handler, cancel.clone()),
            JobSpec::Interval {
                period,
                initial_delay,
            } => spawn_interval(id.clone(), period, initial_delay, handler, cancel.clone()),
            JobSpec::Cron { expression } => {
                spawn_cron(id.clone(), expression, handler, cancel.clone())?
            }
        };

        self.inner.jobs.lock().await.insert(
            id.clone(),
            Entry {
                kind,
                cancel,
                handle,
            },
        );
        Ok(id)
    }

    /// Cancel a scheduled job by id. Returns `true` if the id was known,
    /// `false` if unknown (already completed / cancelled / never existed).
    pub async fn cancel(&self, id: &JobId) -> bool {
        let Some(entry) = self.inner.jobs.lock().await.remove(id) else {
            return false;
        };
        entry.cancel.cancel();
        entry.handle.abort();
        true
    }

    /// Snapshot every currently scheduled job. Cheap read path for UI.
    pub async fn list(&self) -> Vec<JobSnapshot> {
        self.inner
            .jobs
            .lock()
            .await
            .iter()
            .map(|(id, entry)| JobSnapshot {
                id: id.clone(),
                kind: entry.kind,
            })
            .collect()
    }

    fn allocate_id(&self, kind: JobKind) -> JobId {
        let n = self
            .inner
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let prefix = match kind {
            JobKind::OneShot => "oneshot",
            JobKind::Interval => "interval",
            JobKind::Cron => "cron",
        };
        JobId(format!("{prefix}-{n}"))
    }
}

fn spawn_oneshot(
    id: JobId,
    delay: Duration,
    handler: Arc<dyn JobHandler>,
    cancel: tokio_util::sync::CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            () = cancel.cancelled() => return,
        }
        handler.run(id).await;
    })
}

fn spawn_interval(
    id: JobId,
    period: Duration,
    initial_delay: Duration,
    handler: Arc<dyn JobHandler>,
    cancel: tokio_util::sync::CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::select! {
            () = tokio::time::sleep(initial_delay) => {}
            () = cancel.cancelled() => return,
        }
        let mut ticker = tokio::time::interval(period);
        // Skip the immediate first tick — interval fires right away by default.
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    handler.run(id.clone()).await;
                }
                () = cancel.cancelled() => break,
            }
        }
    })
}

fn spawn_cron(
    id: JobId,
    expression: String,
    handler: Arc<dyn JobHandler>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<JoinHandle<()>, ScheduleError> {
    use std::str::FromStr as _;

    let cron =
        croner::Cron::from_str(&expression).map_err(|source| ScheduleError::InvalidCron {
            expression: expression.clone(),
            source,
        })?;
    // Fail fast if the expression somehow never fires (defensive — croner
    // usually errors upfront for truly impossible exprs, but we still
    // validate the first fire is computable).
    cron.find_next_occurrence(&chrono::Utc::now(), false)
        .map_err(|_| ScheduleError::CronWithoutNextFire(expression.clone()))?;

    let handle = tokio::spawn(async move {
        loop {
            let now = chrono::Utc::now();
            let Ok(next) = cron.find_next_occurrence(&now, false) else {
                // Expression no longer yields future fires — log and stop.
                tracing::warn!(
                    job = %id,
                    expression,
                    "cron has no further occurrences; stopping"
                );
                return;
            };
            let wait = (next - now).to_std().unwrap_or(Duration::from_millis(0));
            tokio::select! {
                () = tokio::time::sleep(wait) => {
                    handler.run(id.clone()).await;
                }
                () = cancel.cancelled() => return,
            }
        }
    });
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::FnHandler;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counter_handler() -> (Arc<AtomicUsize>, Arc<dyn JobHandler>) {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let handler: Arc<dyn JobHandler> = Arc::new(FnHandler(move |_id| {
            let c = Arc::clone(&counter_clone);
            async move {
                c.fetch_add(1, Ordering::Relaxed);
            }
        }));
        (counter, handler)
    }

    /// Yield multiple times — one yield is not always enough for the
    /// spawned task to progress past its `sleep`/`tick` in a paused runtime.
    async fn yield_a_bunch() {
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test]
    async fn oneshot_fires_exactly_once() {
        let sched = JobScheduler::new();
        let (counter, handler) = counter_handler();
        let _id = sched
            .schedule(
                JobSpec::OneShot {
                    delay: Duration::from_millis(20),
                },
                handler,
            )
            .await
            .unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 0);
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        tokio::time::sleep(Duration::from_millis(80)).await;
        assert_eq!(counter.load(Ordering::Relaxed), 1, "must not fire again");
    }

    #[tokio::test]
    async fn interval_fires_multiple_times() {
        let sched = JobScheduler::new();
        let (counter, handler) = counter_handler();
        let _id = sched
            .schedule(
                JobSpec::Interval {
                    period: Duration::from_millis(20),
                    initial_delay: Duration::from_millis(20),
                },
                handler,
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(
            counter.load(Ordering::Relaxed) >= 3,
            "expected ≥3 fires, got {}",
            counter.load(Ordering::Relaxed)
        );
    }

    // Placeholder so yield_a_bunch stays reachable as a helper; tests
    // above switched to real-time sleeps, but leaving the helper handy
    // for future `start_paused` tests on cron fires.
    #[tokio::test]
    async fn yield_helper_does_not_panic() {
        yield_a_bunch().await;
    }

    #[tokio::test]
    async fn cancel_stops_future_fires() {
        let sched = JobScheduler::new();
        let (counter, handler) = counter_handler();
        let id = sched
            .schedule(
                JobSpec::Interval {
                    period: Duration::from_millis(10),
                    initial_delay: Duration::from_millis(10),
                },
                handler,
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        let cancelled = sched.cancel(&id).await;
        assert!(cancelled);
        let before = counter.load(Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(100)).await;
        let after = counter.load(Ordering::Relaxed);
        // After cancel, the counter must not continue to climb.
        assert_eq!(before, after, "cancelled interval kept firing");
    }

    #[tokio::test]
    async fn cancel_unknown_returns_false() {
        let sched = JobScheduler::new();
        assert!(!sched.cancel(&JobId("not-a-real-id".into())).await);
    }

    #[tokio::test]
    async fn list_reflects_scheduled_jobs() {
        let sched = JobScheduler::new();
        let (_, h) = counter_handler();
        let id1 = sched
            .schedule(
                JobSpec::OneShot {
                    delay: Duration::from_secs(60),
                },
                Arc::clone(&h),
            )
            .await
            .unwrap();
        let id2 = sched
            .schedule(
                JobSpec::Interval {
                    period: Duration::from_secs(60),
                    initial_delay: Duration::from_secs(60),
                },
                h,
            )
            .await
            .unwrap();

        let list = sched.list().await;
        let ids: Vec<JobId> = list.iter().map(|s| s.id.clone()).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn cron_invalid_expression_errors() {
        let sched = JobScheduler::new();
        let (_, h) = counter_handler();
        let err = sched
            .schedule(
                JobSpec::Cron {
                    expression: "not a cron".into(),
                },
                h,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, ScheduleError::InvalidCron { .. }),
            "expected InvalidCron, got {err:?}"
        );
    }
}
