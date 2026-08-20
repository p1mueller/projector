use std::ops::DerefMut;

use crate::{
    event::{AppEvent, Event, EventHandler},
    forms::{GetForm, ProjectForm},
    project::{Project, ProjectError, ProjectHandler},
    widgets::filter::FilterState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

// TODO: Less file IO (reloading only when explicit or needed)

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Home,
    Add,
    Edit,
    Remove,
    Filter,
    Error(String),
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Project handler.
    pub project_handler: ProjectHandler,
    pub list_state: ListState,
    /// Event handler.
    pub events: EventHandler,
    pub state: AppState,
}

#[derive(Debug)]
pub struct AppState {
    pub needs_redraw: bool,
    pub needs_reload: bool,
    pub failed: bool,
    pub show_group: bool,
    pub mode: Mode,
    pub overview: ListState,
    pub project_form: ProjectForm,
    pub filter: FilterState,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> color_eyre::Result<Self> {
        Ok(Self {
            running: true,
            project_handler: ProjectHandler::default(),
            list_state: ListState::default(),
            events: EventHandler::new()?,
            state: AppState::default(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self) -> color_eyre::Result<()> {
        self.events.enter()?;
        self.list_state.select_first();
        while self.running {
            if self.state.needs_reload {
                match self.project_handler.read_config() {
                    Ok(_) => self.state.failed = false,
                    Err(error) => {
                        if self.state.failed {
                            let res = self.open_editor(ProjectHandler::edit_settings);
                            if let Err(error) = res {
                                println!("{error}");
                                return Err(error);
                            }
                        } else {
                            let msg = format!(
                                "Encountered error while reading configuration:\n{error}\nYou will be forwarded to your editor to fix the issue."
                            );
                            self.state.mode = Mode::Error(msg);
                            self.state.failed = true;
                        }
                    }
                }
                self.state.needs_reload = false;
                self.state.needs_redraw = true;
            }

            if self.state.needs_redraw {
                let project_handler = &self.project_handler;
                let state = &mut self.state;

                self.events.draw(|frame| {
                    Self::render(project_handler, frame.area(), frame.buffer_mut(), state);
                })?;
                self.state.needs_redraw = false;
            }

            self.handle_events().await?;
        }
        Ok(())
    }

    pub async fn handle_events(&mut self) -> color_eyre::Result<()> {
        match self.events.next().await? {
            Event::Crossterm(event) => match event {
                crossterm::event::Event::Key(key_event)
                    if key_event.kind == crossterm::event::KeyEventKind::Press =>
                {
                    self.handle_key_event(key_event)?
                }
                _ => {
                    self.state.needs_redraw = true;
                }
            },
            Event::App(app_event) => {
                self.state.needs_redraw = true;
                if app_event == AppEvent::Quit {
                    self.quit();
                    return Ok(());
                }
                let overview = &mut self.state.overview;
                let project_state = self.state.project_form.state_mut();
                match self.state.mode {
                    Mode::Home => match app_event {
                        AppEvent::SelectFirst => overview.select_first(),
                        AppEvent::SelectLast => overview.select_last(),
                        AppEvent::SelectNext => overview.select_next(),
                        AppEvent::SelectPrevious => overview.select_previous(),
                        AppEvent::Unselect => overview.select(None),
                        AppEvent::ToggleGroup => self.state.show_group ^= true,
                        AppEvent::AddProject => {
                            if self.state.failed {
                                return Ok(());
                            }
                            self.state.mode = Mode::Add;
                            project_state.select_first();
                        }
                        AppEvent::EditProject => {
                            if self.state.failed {
                                return Ok(());
                            }
                            if let Some(project) = self.selected_project().cloned() {
                                self.state.mode = Mode::Edit;
                                self.state.project_form.set_project(project);
                                self.state.project_form.state_mut().select_first();
                            }
                        }
                        AppEvent::RemoveProject => {
                            if self.selected_project().is_some() {
                                self.state.mode = Mode::Remove;
                            }
                        }
                        AppEvent::LaunchProject => {
                            if let Some(project) = self.selected_project() {
                                let result = self.project_handler.launch_project(project);
                                if let Err(err) = result {
                                    self.state.mode = Mode::Error(err.to_string())
                                }
                            }
                        }
                        AppEvent::EditSettings => {
                            if let Err(err) = self.open_editor(ProjectHandler::edit_settings) {
                                self.state.mode = Mode::Error(err.to_string())
                            }
                        }
                        AppEvent::EditScript => {
                            let project = self.selected_project().cloned();

                            if let Some(project) = project
                                && let Err(err) =
                                    self.open_editor(|ph| ph.edit_project_script(&project))
                            {
                                self.state.mode = Mode::Error(err.to_string());
                            }
                        }
                        AppEvent::FilterProject => {
                            self.state.mode = Mode::Filter;
                            self.state.filter.active = true;
                        }
                        AppEvent::Reload => {
                            self.state.failed = false;
                            self.state.needs_reload = true;
                        }
                        AppEvent::Abort => self.go_home(),
                        _ => self.state.needs_redraw = false,
                    },

                    Mode::Add | Mode::Edit | Mode::Filter => {
                        let state = match self.state.mode {
                            Mode::Add | Mode::Edit => project_state.deref_mut(),
                            Mode::Filter => self.state.filter.deref_mut(),
                            _ => unreachable!(),
                        };
                        match app_event {
                            AppEvent::SelectNext => project_state.select_next(),
                            AppEvent::SelectPrevious => project_state.select_previous(),
                            AppEvent::MoveLeft => state.move_cursor_left(),
                            AppEvent::MoveRight => state.move_cursor_right(),
                            AppEvent::Char(ch) => state.put(ch),
                            AppEvent::Backspace => state.backspace(),
                            AppEvent::Submit => match self.state.mode {
                                Mode::Add | Mode::Edit => {
                                    self.handle_form_submit()?;
                                }
                                Mode::Filter => {
                                    self.handle_filter_submit()?;
                                }
                                _ => unreachable!(),
                            },
                            AppEvent::Abort => match self.state.mode {
                                Mode::Add | Mode::Edit => {
                                    project_state.clear_all();
                                    self.state.mode = Mode::Home;
                                }
                                Mode::Filter => {
                                    self.go_home();
                                }
                                _ => unreachable!(),
                            },
                            _ => self.state.needs_redraw = false,
                        }
                    }

                    Mode::Remove => match app_event {
                        AppEvent::Submit => {
                            let project = self.selected_project().unwrap().clone();
                            let result = self.project_handler.remove_project(&project);
                            match result {
                                Ok(_) => {
                                    self.project_handler.write_config()?;
                                    self.go_home();
                                }

                                Err(err) => self.state.mode = Mode::Error(err.to_string()),
                            }
                        }
                        AppEvent::Abort => self.state.mode = Mode::Home,
                        _ => self.state.needs_redraw = false,
                    },

                    Mode::Error(_) => {
                        if app_event == AppEvent::Submit {
                            self.state.needs_reload = true;
                            self.go_home();
                        } else {
                            self.state.needs_redraw = false;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match self.state.mode {
            Mode::Home => match key_event.code {
                KeyCode::Esc => {
                    if self.state.filter.active {
                        self.events.send(AppEvent::Abort)
                    } else {
                        self.events.send(AppEvent::Quit)
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => self.events.send(AppEvent::LaunchProject),
                KeyCode::Char('g') => self.events.send(AppEvent::SelectFirst),
                KeyCode::Char('G') => self.events.send(AppEvent::SelectLast),
                KeyCode::Down | KeyCode::Char('j') => self.events.send(AppEvent::SelectNext),
                KeyCode::Up | KeyCode::Char('k') => self.events.send(AppEvent::SelectPrevious),
                KeyCode::Left | KeyCode::Char('h') => self.events.send(AppEvent::Unselect),
                KeyCode::Right | KeyCode::Char('l') => self.events.send(AppEvent::EditProject),
                KeyCode::Char('e') => self.events.send(AppEvent::EditScript),
                KeyCode::Char('a' | 'n') => self.events.send(AppEvent::AddProject),
                KeyCode::Char('d') => self.events.send(AppEvent::RemoveProject),
                KeyCode::Char('i') => self.events.send(AppEvent::ToggleGroup),
                KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.events.send(AppEvent::Quit)
                }
                KeyCode::Char('s') => self.events.send(AppEvent::EditSettings),
                KeyCode::Char('f' | 'F' | '/') => self.events.send(AppEvent::FilterProject),
                KeyCode::Char('r' | 'R') => self.events.send(AppEvent::Reload),
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                _ => {}
            },

            Mode::Add | Mode::Edit | Mode::Filter => match key_event.code {
                KeyCode::Esc => self.events.send(AppEvent::Abort),
                KeyCode::Down | KeyCode::Tab => {
                    if self.state.mode == Mode::Filter {
                        self.events.send(AppEvent::Submit)
                    } else {
                        self.events.send(AppEvent::SelectNext)
                    }
                }
                KeyCode::Up | KeyCode::BackTab => self.events.send(AppEvent::SelectPrevious),
                KeyCode::Left => self.events.send(AppEvent::MoveLeft),
                KeyCode::Right => self.events.send(AppEvent::MoveRight),
                KeyCode::Char(ch) => self.events.send(AppEvent::Char(ch)),
                KeyCode::Backspace => self.events.send(AppEvent::Backspace),
                KeyCode::Enter => self.events.send(AppEvent::Submit),
                _ => {}
            },

            Mode::Remove => match key_event.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y') => self.events.send(AppEvent::Submit),
                KeyCode::Esc | KeyCode::Char('n' | 'N') => self.events.send(AppEvent::Abort),
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                _ => {}
            },

            Mode::Error(_) => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                _ => self.events.send(AppEvent::Submit),
            },
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    fn selected_project(&self) -> Option<&Project> {
        let index = self.state.overview.selected()?;

        if self.state.filter.active {
            self.project_handler
                .filter_projects(self.state.filter.text())
                .nth(index)
        } else {
            self.project_handler.projects().get(index)
        }
    }

    fn open_editor<F>(&mut self, edit: F) -> color_eyre::Result<()>
    where
        F: FnOnce(&ProjectHandler) -> Result<(), ProjectError>,
    {
        self.events.exit()?;

        let result = edit(&self.project_handler);

        self.events.enter()?;
        self.events.clear()?;

        self.state.failed = false;
        self.events.send(AppEvent::Reload);

        result?;

        Ok(())
    }

    fn handle_form_submit(&mut self) -> color_eyre::Result<()> {
        let request = self.state.project_form.get_project();
        let result = match self.state.mode {
            Mode::Add => self.project_handler.add_project(request),
            Mode::Edit => {
                let project = self.selected_project().unwrap().clone();
                self.project_handler.edit_project(&project, request)
            }
            _ => unreachable!(),
        };
        match result {
            Ok(_) => {
                self.project_handler.write_config()?;
                self.state.project_form.state_mut().clear_all();
                self.go_home();
            }
            Err(error) => self.state.project_form.state_mut().set_error(error),
        }
        Ok(())
    }

    fn handle_filter_submit(&mut self) -> color_eyre::Result<()> {
        self.state.mode = Mode::Home;
        if self.state.filter.text().is_empty() {
            self.state.filter.active = false;
        }
        self.state.overview.select_first();
        Ok(())
    }

    fn go_home(&mut self) {
        self.state.mode = Mode::Home;
        self.state.filter.clear_all();
    }
}

impl Default for AppState {
    fn default() -> Self {
        let mut overview = ListState::default();
        overview.select_first();
        Self {
            needs_redraw: true,
            needs_reload: true,
            failed: false,
            show_group: false,
            mode: Mode::default(),
            overview,
            project_form: ProjectForm::default(),
            filter: FilterState::default(),
        }
    }
}
