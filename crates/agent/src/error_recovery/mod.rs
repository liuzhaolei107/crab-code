//! Error classification and recovery strategy — aligned with CCB's
//! `classifyAPIError()` approach from `src/services/api/errors.ts`.
//!
//! Two modules:
//!
//! - [`category`] — [`ErrorCategory`] + [`ErrorClassifier`] map error text
//!   or HTTP status to a small enum (`Transient` / `RateLimit` / `Auth` /
//!   `Timeout` / `Permanent` / `Unknown`).
//! - [`strategy`] — [`RecoveryAction`] + [`RecoveryStrategy`] pick
//!   `Retry` / `AskUser` / `Abort` per category.
//!
//! The earlier `circuit` (`CircuitBreaker`) and `degradation`
//! (`GracefulDegradation`) modules were removed in Phase 4.1 — CCB does
//! not ship equivalent abstractions, and crab's versions were
//! unintegrated.

pub mod category;
pub mod strategy;

pub use category::{ErrorCategory, ErrorClassifier};
pub use strategy::{RecoveryAction, RecoveryStrategy};
