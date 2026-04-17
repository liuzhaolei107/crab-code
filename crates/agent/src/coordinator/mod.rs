//! Multi-agent orchestration core.
//!
//! - [`session_config`] — [`SessionConfig`] value struct for session startup
//! - [`session`] — [`AgentSession`] running-session state + event plumbing
//! - [`manager`] — [`AgentCoordinator`] + [`AgentHandle`] for sub-agent workers

pub mod manager;
pub mod session;
pub mod session_config;

pub use manager::{AgentCoordinator, AgentHandle};
pub use session::AgentSession;
pub use session_config::SessionConfig;
