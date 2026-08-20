//! Reusable ratatui widgets: text input, filters, forms, popups, and their
//! shared scroll state.
//!
//! [`scroll_state::ScrollState`] is the cursor/window tracking shared between
//! [`text_input::TextInput`] and higher-level widgets.

pub mod filter;
pub mod form;
pub mod popup;
pub mod scroll_state;
pub mod text_input;
