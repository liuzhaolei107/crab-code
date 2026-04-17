//! Crab-proto server side — accept remote clients that attach to a
//! running crab session.
//!
//! This module currently ships the configuration types that α.4.b will
//! bolt a real axum listener onto. Keeping the config in this commit
//! means downstream consumers (`daemon`) can already start loading /
//! validating their `~/.crab/settings.json` even before the listener
//! exists.

pub mod config;

pub use config::{ServerConfig, ServerConfigError};
