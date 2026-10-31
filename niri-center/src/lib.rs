//! `niri-center`: everything the app does that is not drawing it. The
//! config read and edited losslessly, saving with its history, the table
//! of settings, what niri and the system report, and the words for all of
//! it. `niri-center-ui` draws it with iced; nothing in here knows about
//! widgets.

pub mod animations;
pub mod backup;
pub mod bindings;
pub mod by_app;
pub mod cli;
pub mod clock;
pub mod color;
pub mod config;
pub mod demo;
pub mod diff;
pub mod disk;
pub mod forms;
pub mod highlight;
pub mod history;
pub mod include;
#[cfg(not(target_arch = "wasm32"))]
pub mod ipc;
pub mod kdl;
pub mod keyboard;
pub mod library;
pub mod motion;
pub mod niri;
pub mod outputs;
pub mod page;
pub mod portal;
pub mod prefs;
pub mod rules;
pub mod sandbox;
pub mod search;
pub mod session;
pub mod settings;
pub mod snapshot;
pub mod start;
pub mod store;
pub mod summary;
pub mod system;
pub mod typo;
pub mod validate;
pub mod words;
pub mod xdg;
