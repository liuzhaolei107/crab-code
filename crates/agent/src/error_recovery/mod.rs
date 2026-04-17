//! Error recovery: classification, strategy selection, circuit breaker,
//! and graceful degradation for resilient agent operation.
//!
//! Builds on the retry module to provide higher-level error handling
//! that adapts behaviour based on error type and failure patterns.
//!
//! ## Sub-modules
//!
//! - [`category`]    — [`ErrorCategory`] + [`ErrorClassifier`]
//! - [`strategy`]    — [`RecoveryAction`] + [`RecoveryStrategy`]
//! - [`circuit`]     — [`CircuitState`] + [`CircuitBreaker`] (+ config)
//! - [`degradation`] — [`DegradableFeature`] + [`FeaturePriority`] + [`GracefulDegradation`]

pub mod category;
pub mod circuit;
pub mod degradation;
pub mod strategy;

pub use category::{ErrorCategory, ErrorClassifier};
pub use circuit::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use degradation::{DegradableFeature, FeaturePriority, GracefulDegradation};
pub use strategy::{RecoveryAction, RecoveryStrategy};
