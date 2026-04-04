//! OpenAI-compatible Chat Completions API — complete independent implementation.

pub mod client;
pub mod convert;
pub mod types;

pub use client::OpenAiClient;
