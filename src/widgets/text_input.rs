use super::scroll_state::ScrollState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

const CURSOR_STYLE: Style = Style::new().fg(Color::Black).bg(Color::LightBlue);
const LINE_STYLE: Style = Style::new();

#[derive(Debug)]
pub struct TextInput {
    line_style: Style,
    cursor_style: Style,
}

#[derive(Debug, Default)]
pub struct TextInputState {
    text: String,
    scroll_state: ScrollState,
}

impl StatefulWidget for TextInput {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.scroll_state.set_viewport_length(area.width as usize);

        let start_idx = state.scroll_state.start_position();
        let highlight_idx =
            get_utf8_index(&state.text, state.cursor_position()).unwrap_or(state.text.len());

        let highlight = state.text.get(highlight_idx..=highlight_idx).unwrap_or(" ");
        let before_highlight = state.text.get(start_idx..highlight_idx).unwrap_or(" ");
        let after_highlight = state.text.get(highlight_idx + 1..).unwrap_or(" ");

        let text_components = vec![
            Span::from(before_highlight),
            Span::from(highlight).style(self.cursor_style),
            Span::from(after_highlight),
        ];
        let line = Line::from(text_components).style(self.line_style);
        line.render(area, buf);
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            line_style: LINE_STYLE,
            cursor_style: CURSOR_STYLE,
        }
    }
}

impl TextInputState {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.scroll_state.set_content_length(1);
    }

    pub fn put(&mut self, ch: char) {
        let idx = get_utf8_index(&self.text, self.cursor_position()).unwrap_or(self.text.len());
        self.text.insert(idx, ch);
        self.scroll_state.set_content_length(self.text.len() + 1);
        self.scroll_state.next();
    }

    pub fn backspace(&mut self) {
        if self.cursor_position() >= 1 {
            let idx = get_utf8_index(&self.text, self.cursor_position() - 1).unwrap();
            self.text.remove(idx);
            self.scroll_state.prev();
            self.scroll_state.set_content_length(self.text.len() + 1);
        }
    }

    pub fn move_cursor_right(&mut self) {
        self.scroll_state.next();
    }

    pub fn move_cursor_left(&mut self) {
        self.scroll_state.prev();
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.scroll_state.reset_content(self.text.len() + 1);
    }

    fn cursor_position(&self) -> usize {
        self.scroll_state.start_position() + self.scroll_state.selected_position()
    }
}

impl TextInput {
    pub fn new(line_style: Style, cursor_style: Style) -> Self {
        Self {
            line_style,
            cursor_style,
        }
    }

    pub fn line_style(mut self, style: Style) -> Self {
        self.line_style = style;
        self
    }

    pub fn cursor_style(mut self, style: Style) -> Self {
        self.cursor_style = style;
        self
    }
}

fn get_utf8_index(s: &str, ch_idx: usize) -> Option<usize> {
    s.char_indices().nth(ch_idx).map(|(i, _)| i)
}
