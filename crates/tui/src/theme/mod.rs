pub mod accents;
pub mod agents;
mod config;
pub mod current;
pub mod osc;
mod parse;
mod registry;
pub mod shimmer;
#[allow(clippy::module_inception)]
mod theme;

pub use accents::Accents;
pub use agents::{AGENTS_PALETTE_DARK, AGENTS_PALETTE_LIGHT, agent_color};
pub use config::{ThemeConfig, load_theme_config};
pub use current::{current, init as init_current};
pub use osc::{Detection, detect_background};
pub use registry::ThemeRegistry;
pub use shimmer::{SHIMMER_INTERVAL, SHIMMER_INTERVAL_MS, shimmer_at, shimmer_segments};
pub use theme::{Theme, ThemeName};
