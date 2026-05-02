//! Platform-specific screen/input backends for Computer Use.
//!
//! Each backend is feature-gated so that building without the feature
//! avoids pulling in heavy system dependencies (Win32, `XCap`, enigo, etc.).
//! Populated incrementally. Phase 4 scaffold only.

#[cfg(all(target_os = "macos", feature = "macos-ax"))]
pub mod macos;

#[cfg(all(target_os = "windows", feature = "win-native"))]
pub mod windows;

#[cfg(all(target_os = "linux", any(feature = "x11", feature = "wayland")))]
pub mod linux;
