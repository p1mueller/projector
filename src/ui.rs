use std::path::PathBuf;

use crate::{
    app::{App, AppState, Mode},
    forms::GetForm,
    project::{Project, ProjectHandler},
    widgets::{form::Form, popup::get_popup_area_centered},
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, HorizontalAlignment, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
        StatefulWidget, Widget,
    },
};

const ITEM_STYLE: Style = Style::new().bold();
const VALUE_STYLE: Style = Style::new();
const PARENT_STYLE: Style = Style::new().bold().dim();
const SELECTED_STYLE: Style = ITEM_STYLE.bg(Color::DarkGray);
const ERROR_STYLE: Style = ITEM_STYLE.fg(Color::Red);
const POPUP_STYLE: Style = ITEM_STYLE.fg(Color::Blue);

impl App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    pub(super) fn render(
        project_handler: &ProjectHandler,
        area: Rect,
        buf: &mut Buffer,
        state: &mut AppState,
    ) {
        let outer_layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]);
        let [main_area, footer_area] = area.layout(&outer_layout);

        let main_layout = Layout::horizontal([Constraint::Max(50), Constraint::Fill(2)]);
        let [left_area, preview_area] = main_area.layout(&main_layout);

        let left_layout = Layout::vertical([Constraint::Fill(1), Constraint::Max(5)]);
        let [overview_area, detail_area] = left_area.layout(&left_layout);

        Self::render_overview(project_handler, overview_area, buf, state);
        Self::render_detail_view(project_handler, detail_area, buf, state);
        Self::render_preview(project_handler, preview_area, buf, state);
        Self::render_footer(footer_area, buf);

        match &state.mode {
            Mode::Add => Form::new("Add Project").render(area, buf, state.project_form.state_mut()),

            Mode::Edit => {
                Form::new("Edit Project").render(area, buf, state.project_form.state_mut())
            }

            Mode::Remove => render_popup(
                "",
                "Do you really want to remove this object? [Y/n]",
                POPUP_STYLE,
                area,
                buf,
            ),

            Mode::Error(msg) => render_popup("Error", msg, ERROR_STYLE, area, buf),
            // Mode::Error => Form::styled("Error", ERROR_STYLE).render(
            //     area,
            //     buf,
            //     self.state.error_form.state_mut(),
            // ),
            _ => {}
        }
    }

    fn render_overview(
        project_handler: &ProjectHandler,
        area: Rect,
        buf: &mut Buffer,
        state: &mut AppState,
    ) {
        let block = create_block(" Projects ");
        let overview_list = List::new(project_handler.projects())
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(&overview_list, area, buf, &mut state.overview);
    }

    fn render_detail_view(
        project_handler: &ProjectHandler,
        area: Rect,
        buf: &mut Buffer,
        state: &AppState,
    ) {
        let block = create_block(" Details ");
        let path: PathBuf;
        let lines =
            if let Some(project) = Self::get_selected_project(project_handler, &state.overview) {
                let parent = project.parent.as_ref().map_or_else(|| "", |x| x.as_str());
                path = project_handler.script_path(project);
                vec![
                    Line::from_iter([
                        Span::styled("Name:   ", ITEM_STYLE),
                        Span::styled(&project.name, VALUE_STYLE),
                    ]),
                    Line::from_iter([
                        Span::styled("Parent: ", ITEM_STYLE),
                        Span::styled(parent, VALUE_STYLE),
                    ]),
                    Line::from_iter([
                        Span::styled("Script: ", ITEM_STYLE),
                        Span::styled(path.to_str().unwrap(), VALUE_STYLE),
                    ]),
                ]
            } else {
                Vec::new()
            };
        let text = Text::from(lines);
        let details = Paragraph::new(text).block(block);
        details.render(area, buf);
    }

    fn render_preview(
        project_handler: &ProjectHandler,
        area: Rect,
        buf: &mut Buffer,
        state: &AppState,
    ) {
        let block = create_block(" Preview ");
        let content = match Self::get_selected_project(project_handler, &state.overview) {
            Some(project) => {
                let path = project_handler.script_path(project);
                std::fs::read_to_string(&path).map_err(|_| path)
            }
            None => Ok("".to_owned()),
        };
        let content = match content {
            Ok(text) => Text::from(text),
            Err(path) => Text::styled(
                format!("Script file `{path:?}` does not exist"),
                ERROR_STYLE,
            ),
        };
        let preview = Paragraph::new(content).block(block);
        preview.render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        let footer = Paragraph::new(
            "[→|l] edit [↑↓|jk] navigate  [g] select first [G] select last [←|h] unselect",
        )
        .centered();
        footer.render(area, buf);
    }

    pub(super) fn get_selected_project<'a>(
        project_handler: &'a ProjectHandler,
        list_state: &'a ListState,
    ) -> Option<&'a Project> {
        let index = list_state.selected()?;
        Some(project_handler.get_project(index))
    }
}

// TODO: change so that parent show can be toggled
impl<'a> From<&'a Project> for ListItem<'a> {
    fn from(value: &'a Project) -> Self {
        let icon = value.icon.as_ref().map_or_else(|| "", |i| i.as_str());
        let style = if value.valid { ITEM_STYLE } else { ERROR_STYLE };

        let content = Text::from(vec![Line::from(Span::styled(
            format!(" {icon} {}", value.name),
            style,
        ))]);

        ListItem::new(content)
    }
}

fn create_block(title: &str) -> Block<'_> {
    Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
}

fn render_popup(title: &str, text: &str, style: Style, area: Rect, buf: &mut Buffer) {
    let lines = text.lines();
    let height = lines.clone().count() + 2;
    let width = lines.map(|x| x.len()).max().unwrap_or(1) + 4;
    let popup_area = get_popup_area_centered(area, width as u16, height as u16);
    let block = create_block(title)
        .style(style)
        .title_alignment(HorizontalAlignment::Center);
    Paragraph::new(text).block(block).render(popup_area, buf)
}
