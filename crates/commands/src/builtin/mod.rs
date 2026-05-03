pub mod auth;
pub mod feedback;
pub mod git;
pub mod meta;
pub mod model;
pub mod navigation;
pub mod project;
pub mod session;
pub mod status;

mod all;

pub use all::register_all;
