//! Remote agent sessions hosted by claude.ai — lifecycle, streaming event
//! subscription, and SDK message adaptation.

pub mod manager;
pub mod sdk_adapter;
#[cfg(feature = "session")]
pub mod websocket;
