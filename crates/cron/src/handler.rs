//! Trait for what a job does when it fires.
//!
//! Kept as a Pin<Box<Future>> shape instead of `#[async_trait]` to avoid
//! adding a proc-macro dep to the workspace — matches how `crab-mcp` and
//! `crab-core` define their async traits.

use crate::id::JobId;

/// Called by [`crate::scheduler::JobScheduler`] every time a scheduled
/// job fires.
///
/// Implementations should be cheap to clone (or inside an `Arc`) — the
/// scheduler holds them as `Arc<dyn JobHandler>` so the same handler can
/// be attached to multiple scheduled jobs.
///
/// Errors during `run` are logged by the scheduler but do not cancel
/// the job; transient failures naturally retry on the next fire.
pub trait JobHandler: Send + Sync {
    /// Execute one firing of the job.
    fn run(
        &self,
        job_id: JobId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>;
}

/// A convenience handler for tests and ad-hoc use: wraps an async closure.
///
/// Consumers that want to schedule a quick inline task don't need to
/// define a whole struct for it.
pub struct FnHandler<F>(pub F);

impl<F, Fut> JobHandler for FnHandler<F>
where
    F: Fn(JobId) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    fn run(
        &self,
        job_id: JobId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin((self.0)(job_id))
    }
}
