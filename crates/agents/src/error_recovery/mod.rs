//! Error classification and recovery strategy.
//!
//! Two modules:
//!
//! - [`category`] — [`ErrorCategory`] + [`ErrorClassifier`] map error text
//!   or HTTP status to a small enum (`Transient` / `RateLimit` / `Auth` /
//!   `Timeout` / `Permanent` / `Unknown`).
//! - [`strategy`] — [`RecoveryAction`] + [`RecoveryStrategy`] pick
//!   `Retry` / `AskUser` / `Abort` per category.

pub mod category;
pub mod strategy;

pub use category::{ErrorCategory, ErrorClassifier};
pub use strategy::{RecoveryAction, RecoveryStrategy};
