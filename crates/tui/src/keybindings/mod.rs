//! Configurable, chord-aware keybinding system.
//!
//! Contexts are organized along the focus chain (innermost → outermost),
//! with `KeyContext::Global` as the implicit outermost fallback. Chord
//! sequences like `Ctrl+K Ctrl+S` resolve progressively: feed each key,
//! receive `PendingChord` until the sequence completes, times out, or
//! disambiguates.
//!
//! Load order:
//!
//! 1. [`defaults::defaults`] populates a new [`Resolver`] with the
//!    built-in bindings for every context.
//! 2. [`config::apply_from_path`] layers user overrides from
//!    `~/.crab/keybindings.json` on top.

pub mod config;
pub mod defaults;
mod facade;
pub mod parser;
pub mod resolver;
pub mod sequence;
pub mod types;

pub use config::{UserBindings, apply as apply_user_bindings, apply_from_path};
pub use defaults::defaults;
pub use facade::Keybindings;
pub use parser::{parse_chord, parse_key_code, parse_sequence};
pub use resolver::{DEFAULT_CHORD_TIMEOUT, ResolveOutcome, Resolver, grouped_bindings};
pub use sequence::{KeySequenceParser, SequenceResult};
pub use types::{Action, KeyChord, KeyCombo, KeyContext, Sequence};
