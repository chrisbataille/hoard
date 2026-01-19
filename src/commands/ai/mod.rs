//! AI command implementations
//!
//! Commands for AI-assisted tool management using various AI providers.

mod analyze;
mod bundle;
mod cheatsheet;
mod config;
mod discover;
mod enrich;
mod extract;
mod migrate;

// Re-export all public command functions
pub use analyze::cmd_ai_analyze;
pub use bundle::cmd_ai_suggest_bundle;
pub use cheatsheet::{cmd_ai_bundle_cheatsheet, cmd_ai_cheatsheet, invalidate_cheatsheet_cache};
pub use config::{cmd_ai_model, cmd_ai_set, cmd_ai_show, cmd_ai_test};
pub use discover::cmd_ai_discover;
pub use enrich::{cmd_ai_categorize, cmd_ai_describe};
pub use extract::cmd_ai_extract;
pub use migrate::cmd_ai_migrate;
