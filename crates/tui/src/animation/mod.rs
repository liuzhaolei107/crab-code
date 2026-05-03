//! Frame-driven animation primitives.
//!
//! The UI layer describes animations declaratively; the runner calls
//! [`FrameScheduler::tick`] each frame to advance subscribers and
//! decide whether to request another redraw.
//!
//! Scheduler policy:
//!
//! - When no subscribers are active, the scheduler does nothing and the
//!   runner's idle redraw rate applies.
//! - When subscribers are active, [`FrameScheduler::should_tick`] returns
//!   `true` at most once per `min_interval` (`16ms` → ~60fps by default).
//! - Subscribers are indexed by `Ticket`; dropping the ticket deregisters
//!   automatically, so widgets can own their subscription without extra
//!   bookkeeping.

mod scheduler;
pub mod shimmer;
pub mod spinner;

pub use scheduler::{FrameScheduler, IDLE_FPS_INTERVAL, MAX_FPS_INTERVAL, Ticket};
pub use shimmer::{ShimmerState, ShimmerSubscriber};
pub use spinner::{BRAILLE_FRAMES, DOTS_FRAMES, LINE_FRAMES, Spinner, SpinnerStyle};
