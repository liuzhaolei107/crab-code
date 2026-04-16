//! REST control plane for the bridge server — active when feature
//! `rest-api` is enabled. Provides start / stop / list-session endpoints
//! used by daemons and admin UIs.

pub mod peer_sessions;
pub mod rest;
