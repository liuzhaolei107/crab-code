//! Platform backends. Auto-selection precedence (when feature `auto` is on):
//! `seatbelt` > `landlock` > `wsl` > `noop`.

#[cfg(feature = "noop")]
pub mod noop;

#[cfg(all(target_os = "macos", feature = "seatbelt"))]
pub mod seatbelt;

#[cfg(all(target_os = "linux", feature = "landlock"))]
pub mod landlock;

#[cfg(all(target_os = "windows", feature = "wsl"))]
pub mod wsl;
