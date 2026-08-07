use crate::{
    event::{AppEvent, Event, EventHandler},
    forms::{ErrorForm, GetForm, ProjectForm},
    project::{ProjectError, ProjectHandler},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

//  TODO: Add filter feature

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    #[default]
    Home,
    Add,
    Edit,
    Remove,
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
    pub failed: bool,
    pub show_group: bool,
    pub mode: Mode,
    pub overview: ListState,
    pub project_form: ProjectForm,
    pub error_form: ErrorForm,
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
            if self.state.needs_redraw {
                match self.project_handler.read_config() {
                    Ok(_) => self.state.failed = false,
                    Err(error) => {
                        if self.state.failed {
                            self.open_editor(ProjectHandler::edit_settings)?
                        } else {
                            let msg = format!(
                                "Encountered error while reading configuration:\n{error}\nYou will be forwarded to your editor to fix the issue."
                            );
                            self.state.mode = Mode::Error(msg);
                            self.state.failed = true;
                        }
                    }
                }
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
            Event::Tick => self.tick(),
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
                            if let Some(index) = self.state.overview.selected() {
                                let project = self.project_handler.get_project(index);
                                self.state.mode = Mode::Edit;
                                project_state.select_first();
                                self.state.project_form.set_project(project);
                            }
                        }
                        AppEvent::RemoveProject => {
                            if self.state.overview.selected().is_some() {
                                self.state.mode = Mode::Remove;
                            }
                        }
                        AppEvent::LaunchProject => {
                            if let Some(index) = overview.selected() {
                                let result = self.project_handler.launch_project(index);
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
                            if let Some(index) = overview.selected()
                                && let Err(err) =
                                    self.open_editor(|ph| ph.edit_project_script(index))
                            {
                                self.state.mode = Mode::Error(err.to_string())
                            }
                        }
                        AppEvent::Reload => {
                            self.state.failed = false;
                            self.state.needs_redraw = true;
                        }
                        _ => todo!(),
                    },

                    Mode::Add | Mode::Edit => match app_event {
                        AppEvent::SelectNext => project_state.select_next(),
                        AppEvent::SelectPrevious => project_state.select_previous(),
                        AppEvent::MoveLeft => project_state.move_cursor_left(),
                        AppEvent::MoveRight => project_state.move_cursor_right(),
                        AppEvent::Char(ch) => project_state.put(ch),
                        AppEvent::Backspace => project_state.backspace(),
                        AppEvent::Submit => {
                            let request = self.state.project_form.get_project();
                            let result = match self.state.mode {
                                Mode::Add => self.project_handler.add_project(request),
                                Mode::Edit => {
                                    let index = overview.selected().unwrap();
                                    self.project_handler.edit_project(index, request)
                                }
                                _ => unreachable!(),
                            };
                            match result {
                                Ok(_) => {
                                    self.project_handler.write_config()?;
                                    // self.project_handler.read_config()?;
                                    self.state.project_form.state_mut().clear();
                                    self.state.mode = Mode::Home;
                                }
                                Err(error) => self.state.project_form.state_mut().set_error(error),
                            }
                        }
                        AppEvent::Abort => {
                            self.state.mode = Mode::Home;
                            project_state.clear();
                        }
                        _ => {}
                    },

                    Mode::Remove => match app_event {
                        AppEvent::Submit => {
                            let index = overview.selected().unwrap();
                            let result = self.project_handler.remove_project(index);
                            match result {
                                Ok(_) => {
                                    self.project_handler.write_config()?;
                                    self.state.mode = Mode::Home;
                                }

                                Err(err) => self.state.mode = Mode::Error(err.to_string()),
                            }
                        }
                        AppEvent::Abort => self.state.mode = Mode::Home,
                        _ => {}
                    },

                    Mode::Error(_) => {
                        if app_event == AppEvent::Submit {
                            self.state.error_form.clear();
                            self.state.mode = Mode::Home;
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
                KeyCode::Esc => self.events.send(AppEvent::Quit),
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
                KeyCode::Char('r' | 'R') => self.events.send(AppEvent::Reload),
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                _ => {}
            },

            Mode::Add | Mode::Edit => match key_event.code {
                KeyCode::Esc => self.events.send(AppEvent::Abort),
                KeyCode::Down | KeyCode::Tab => self.events.send(AppEvent::SelectNext),
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

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
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
}

impl Default for AppState {
    fn default() -> Self {
        let mut overview = ListState::default();
        overview.select_first();
        Self {
            needs_redraw: true,
            failed: false,
            show_group: false,
            mode: Mode::default(),
            overview,
            project_form: ProjectForm::default(),
            error_form: ErrorForm::default(),
        }
    }
}
