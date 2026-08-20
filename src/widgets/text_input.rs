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
        self.scroll_state.reset_content(1);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn type_in(state: &mut TextInputState, text: &str) {
        for ch in text.chars() {
            state.put(ch);
        }
    }

    #[test]
    fn put_builds_text() {
        let mut state = TextInputState::default();
        type_in(&mut state, "abc");
        assert_eq!(state.text(), "abc");
        assert_eq!(state.cursor_position(), 3);
    }

    #[test]
    fn backspace_removes_last_char() {
        let mut state = TextInputState::default();
        type_in(&mut state, "abc");
        state.backspace();
        assert_eq!(state.text(), "ab");
        assert_eq!(state.cursor_position(), 2);
    }

    #[test]
    fn backspace_on_empty_is_noop() {
        let mut state = TextInputState::default();
        state.backspace();
        assert_eq!(state.text(), "");
    }

    #[test]
    fn cursor_navigation_and_insert_in_middle() {
        let mut state = TextInputState::default();
        type_in(&mut state, "ac");
        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 1);
        state.put('b');
        assert_eq!(state.text(), "abc");
        state.move_cursor_left();
        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 0);
        state.put('x');
        assert_eq!(state.text(), "xabc");
    }

    #[test]
    fn multibyte_chars_are_handled() {
        let mut state = TextInputState::default();
        type_in(&mut state, "a\u{1F418}"); // a + elephant emoji (2 chars)
        assert_eq!(state.text(), "a\u{1F418}");
        assert_eq!(state.cursor_position(), 2);
        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 1);
        state.put('b'); // inserts between 'a' and the emoji
        assert_eq!(state.text(), "ab\u{1F418}");
        // Backspace must remove the whole emoji, not half of it.
        state.move_cursor_right();
        state.backspace();
        assert_eq!(state.text(), "ab");
        state.backspace();
        assert_eq!(state.text(), "a");
    }

    #[test]
    fn set_text_replaces_content_and_anchors_cursor() {
        let mut state = TextInputState::default();
        type_in(&mut state, "hello");
        state.move_cursor_left();
        state.set_text("world");
        assert_eq!(state.text(), "world");
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn set_text_is_idempotent() {
        let mut state = TextInputState::default();
        state.set_text("same");
        state.set_text("same");
        assert_eq!(state.text(), "same");
        assert_eq!(state.cursor_position(), 0);
    }

    #[test]
    fn clear_empties_text_and_cursor() {
        let mut state = TextInputState::default();
        type_in(&mut state, "something");
        state.clear();
        assert_eq!(state.text(), "");
        assert_eq!(state.cursor_position(), 0);
    }
}
