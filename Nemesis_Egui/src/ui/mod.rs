//! Drawing code.
//!
//! Every view draws from borrowed state and reports what the user did as return
//! values (events); [`crate::app`] applies them. Views never start processes or
//! write files themselves.
//!
//! * [`style`]: fonts and colors;
//! * [`widgets`]: small reusable widgets;
//! * [`nav_bar`]: the Patch/Settings tab bar;
//! * [`directories`]: data/output directory inputs;
//! * [`mod_table`]: the mod list with checkboxes and drag & drop;
//! * [`log_view`]: the engine log;
//! * [`action_bar`]: progress bar, log buttons and the Patch button;
//! * [`settings`]: the Settings tab.

pub mod action_bar;
pub mod directories;
pub mod log_view;
pub mod mod_table;
pub mod nav_bar;
pub mod settings;
pub mod style;
pub mod widgets;
