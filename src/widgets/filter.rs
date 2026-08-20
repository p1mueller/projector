//! A filter input widget: a labeled text box used to narrow a list of items.
//!
//! [`FilterInput`] (a `StatefulWidget`) renders the label plus a
//! [`super::text_input::TextInput`] backed by [`FilterState`],
//! which also carries an `active` flag for keyboard focus.
//!
//! [`FilterState`] derefs to the inner [`TextInputState`] so callers can use
//! the text-input API directly.

use std::ops::{Deref, DerefMut};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::text_input::{TextInput, TextInputState};

/// A single-line text input with a leading label (e.g. `"Filter: "`).
///
/// The label, label style, line style, and cursor style are all settable;
/// the input's state is provided via the [`StatefulWidget`] contract with
/// [`FilterState`].
pub struct FilterInput<'a> {
    /// The label text drawn before the input box.
    indicator: &'a str,
    /// Style applied to the label.
    indicator_style: Style,
    /// Style applied to the input's line (background, etc.).
    line_style: Style,
    /// Style applied to the cursor character.
    cursor_style: Style,
}

/// State for a [`FilterInput`] widget.
///
/// `Deref`/`DerefMut` forward to `input_state` so the filter state can be
/// used wherever a [`TextInputState`] is expected.
#[derive(Debug, Default)]
pub struct FilterState {
    /// The underlying text input state (text, cursor, scroll).
    pub input_state: TextInputState,
    /// Whether the filter currently has keyboard focus / is active.
    pub active: bool,
}

impl StatefulWidget for FilterInput<'_> {
    type State = FilterState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // let block = Block::new().borders(Borders::BOTTOM);
        let indicator_length = self.indicator.len() + 2;
        // let [indicator_area, input_area] = block.inner(area).layout(&Layout::horizontal([
        let [indicator_area, input_area] = area.layout(&Layout::horizontal([
            Constraint::Length(indicator_length as u16),
            Constraint::Fill(1),
        ]));
        let indicator = format!("{}: ", self.indicator);
        Paragraph::new(indicator)
            .style(self.indicator_style)
            .render(indicator_area, buf);
        TextInput::new(self.line_style, self.cursor_style).render(
            input_area,
            buf,
            &mut state.input_state,
        );
    }
}

impl Default for FilterInput<'_> {
    fn default() -> Self {
        Self {
            indicator: "Filter",
            indicator_style: Style::default(),
            line_style: Style::default(),
            cursor_style: Style::default().bg(Color::Blue).fg(Color::Black),
        }
    }
}

impl<'a> FilterInput<'a> {
    /// Create a filter input with the given label, using default styles.
    pub fn new(indicator: &'a str) -> Self {
        let obj = Self::default();
        Self {
            indicator,
            indicator_style: obj.indicator_style,
            line_style: obj.line_style,
            cursor_style: obj.cursor_style,
        }
    }

    /// Create a filter input with explicit styles.
    pub fn styled(
        indicator: &'a str,
        indicator_style: Style,
        line_style: Style,
        cursor_style: Style,
    ) -> Self {
        Self {
            indicator,
            indicator_style,
            line_style,
            cursor_style,
        }
    }
}

// Forward text-input operations to the inner `TextInputState`.
impl Deref for FilterState {
    type Target = TextInputState;

    fn deref(&self) -> &Self::Target {
        &self.input_state
    }
}

impl DerefMut for FilterState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input_state
    }
}

impl FilterState {
    /// Clear the text and deactivate the filter.
    pub fn clear_all(&mut self) {
        self.clear();
        self.active = false;
    }
}
