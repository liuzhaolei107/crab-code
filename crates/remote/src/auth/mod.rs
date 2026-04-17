//! Shared auth primitives for both the client and server sides of
//! crab-proto. Currently hosts JWT issue / verify; trusted-device and
//! per-session secret logic lands alongside the server impl (α.4.b+).

pub mod jwt;

pub use jwt::{Claims, JwtError, sign, verify};
