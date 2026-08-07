use super::{
    popup,
    text_input::{TextInput, TextInputState},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, HorizontalAlignment, Layout, Margin, Rect},
    style::{Color, Style, Styled, Stylize},
    text::Text,
    widgets::{Block, BorderType, Clear, ListState, Paragraph, StatefulWidget, Widget},
};
use std::{collections::HashMap, fmt::Display};

const NO_FOCUS_CURSOR: Style = Style::new().bg(Color::Reset).fg(Color::Reset);
const ERROR_STYLE: Style = Style::new().fg(Color::Red);

pub struct Form<'a> {
    name: &'a str,
    style: Style,
}

#[derive(Debug)]
pub struct FormState {
    pub input_fields: Vec<InputField>,
    pub list_state: ListState,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct InputField {
    name: String,
    state: TextInputState,
}

impl StatefulWidget for Form<'_> {
    type State = FormState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let popup_area = Self::get_popup_area(area, state);

        let block = Block::bordered()
            .title(self.name)
            .title_alignment(HorizontalAlignment::Center)
            .set_style(self.style)
            .border_type(BorderType::Rounded);

        let inner_area = block.inner(popup_area);
        let mut layout_items = vec![Constraint::Length(3); state.input_fields.len()];
        layout_items.push(Constraint::Length(1));
        let input_field_areas = Layout::vertical(layout_items).split(inner_area);
        Clear.render(popup_area, buf);
        block.render(popup_area, buf);

        let index = state.get_selected_index();
        for (i, (input_field, area)) in state
            .input_fields
            .iter_mut()
            .zip(input_field_areas.as_ref())
            .enumerate()
        {
            let highlight = matches!(index, Some(x) if x == i);
            Self::render_input_field(*area, buf, input_field, highlight)
        }

        Self::render_error(
            *input_field_areas.last().unwrap(),
            buf,
            state.error.as_ref(),
        );
    }
}

impl<'a> Form<'a> {
    pub fn new(name: &'a str) -> Self {
        Form::styled(name, Style::default())
    }

    pub fn styled(name: &'a str, style: Style) -> Self {
        Form { name, style }
    }

    fn get_popup_area(area: Rect, state: &mut FormState) -> Rect {
        let height = state.get_render_height() + 3;
        let width = usize::min(area.width as usize, 50);
        let height = area.height.min(height as u16);
        let width = area.width.min(width as u16);
        popup::get_popup_area_centered(area, width, height)
    }

    fn render_input_field(area: Rect, buf: &mut Buffer, field: &mut InputField, highlight: bool) {
        let mut block = Block::bordered()
            .title(field.name.as_str())
            .border_type(BorderType::Rounded);
        let inner_area = block.inner(area);
        let mut text_input = TextInput::default();

        if highlight {
            block = block.fg(Color::Blue);
        } else {
            text_input = text_input.cursor_style(NO_FOCUS_CURSOR);
        }

        block.render(area, buf);
        text_input.render(inner_area, buf, &mut field.state);
    }

    fn render_error(area: Rect, buf: &mut Buffer, error: Option<&String>) {
        let Some(error) = error else {
            return;
        };

        let text = Text::styled(error, ERROR_STYLE);
        let area = area.inner(Margin::new(1, 0));
        Paragraph::new(text).render(area, buf);
    }
}

impl FormState {
    pub fn new(input_fields: Vec<InputField>) -> Self {
        let mut list_state = ListState::default();
        list_state.select_first();
        Self {
            input_fields,
            list_state,
            error: None,
        }
    }

    pub fn get_render_height(&self) -> usize {
        3 * self.input_fields.len()
    }

    pub fn clear(&mut self) {
        for field in &mut self.input_fields {
            field.state.clear();
        }
        self.select_first();
        self.error = None;
    }

    pub fn set_error(&mut self, error: impl Display) {
        self.error = Some(error.to_string());
    }

    pub fn put(&mut self, ch: char) {
        if let Some(field) = self.get_selected_field_mut() {
            field.state.put(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(field) = self.get_selected_field_mut() {
            field.state.backspace();
        }
    }

    pub fn set_text_selected(&mut self, text: &str) {
        if let Some(field) = self.get_selected_field_mut() {
            field.state.set_text(text);
        }
    }

    pub fn set_text(&mut self, index: usize, text: &str) -> bool {
        if index >= self.input_fields.len() {
            return false;
        }
        self.input_fields[index].set_text(text);
        true
    }

    pub fn move_cursor_left(&mut self) {
        if let Some(field) = self.get_selected_field_mut() {
            field.state.move_cursor_left();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(field) = self.get_selected_field_mut() {
            field.state.move_cursor_right();
        }
    }

    pub fn select_first(&mut self) {
        self.list_state.select_first();
    }

    pub fn select_last(&mut self) {
        let last = self.input_fields.len().saturating_sub(1);
        self.list_state.select(Some(last));
    }

    pub fn select_next(&mut self) {
        self.list_state.select_next();
        if let Some(index) = self.list_state.selected()
            && index >= self.input_fields.len()
        {
            self.select_last();
        }
    }

    pub fn select_previous(&mut self) {
        self.list_state.select_previous();
    }

    pub fn unselect(&mut self) {
        self.list_state.select(None);
    }

    pub fn entries(&self) -> HashMap<String, String> {
        self.input_fields
            .iter()
            .map(|f| (f.name.to_lowercase(), f.state.text().to_string()))
            .collect()
    }

    fn get_selected_index(&mut self) -> Option<usize> {
        let index = self.list_state.selected()?;
        if index >= self.input_fields.len() {
            self.select_last();
            self.list_state.selected()
        } else {
            Some(index)
        }
    }

    fn get_selected_field_mut(&mut self) -> Option<&mut InputField> {
        let index = self.get_selected_index()?;
        self.input_fields.get_mut(index)
    }
}

impl InputField {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            state: TextInputState::default(),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.state.set_text(text);
    }

    pub fn text(&self) -> &str {
        self.state.text()
    }
}
