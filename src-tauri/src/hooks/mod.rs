//! Hooks injected only into sessions the app opens (via `claude --settings`), never globally.

pub mod payload;
pub mod server;
pub mod settings_file;
pub mod tokens;
