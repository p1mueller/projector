use std::ops::{Deref, DerefMut};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::text_input::{TextInput, TextInputState};

pub struct FilterInput<'a> {
    indicator: &'a str,
    indicator_style: Style,
    line_style: Style,
    cursor_style: Style,
}

#[derive(Debug, Default)]
pub struct FilterState {
    pub input_state: TextInputState,
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
    pub fn new(indicator: &'a str) -> Self {
        let obj = Self::default();
        Self {
            indicator,
            indicator_style: obj.indicator_style,
            line_style: obj.line_style,
            cursor_style: obj.cursor_style,
        }
    }

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
    pub fn clear_all(&mut self) {
        self.clear();
        self.active = false;
    }
}
