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
    /// The pending status-expiration task, if any. Kept singular so a new status replaces it.
    pub status_task: Option<tokio::task::JoinHandle<()>>,
    pub state: AppState,
}

#[derive(Debug)]
pub struct AppState {
    pub needs_redraw: bool,
    pub needs_reload: bool,
    pub failed: bool,
    pub show_group: bool,
    pub mode: Mode,
    /// Status message (with style), shown in the footer until its TTL lapses.
    pub status: Option<(String, ratatui::style::Style)>,
    /// Generation of the current status; expirations carrying a different value are ignored.
    pub status_generation: usize,
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
            status_task: None,
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

    /// Handle a single event from the [`EventHandler`], updating the app state accordingly.
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
                match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::LaunchFinished {
                        success,
                        code,
                        stdout,
                        stderr,
                    } => self.report_launch_result(success, code, stdout, stderr),
                    AppEvent::StatusExpired(generation) => self.expire_status(generation),
                    event => self.handle_app_event(event)?,
                }
            }
        }
        Ok(())
    }

    /// Route an application event to the handler for the current mode.
    fn handle_app_event(&mut self, event: AppEvent) -> color_eyre::Result<()> {
        match self.state.mode {
            Mode::Home => self.handle_home_event(event),
            Mode::Add | Mode::Edit => self.handle_project_form_event(event),
            Mode::Filter => self.handle_filter_event(event),
            Mode::Remove => self.handle_remove_event(event),
            Mode::Error(_) => self.handle_error_event(event),
        }
    }

    fn handle_home_event(&mut self, event: AppEvent) -> color_eyre::Result<()> {
        match event {
            AppEvent::SelectFirst => self.state.overview.select_first(),
            AppEvent::SelectLast => self.state.overview.select_last(),
            AppEvent::SelectNext => self.state.overview.select_next(),
            AppEvent::SelectPrevious => self.state.overview.select_previous(),
            AppEvent::Unselect => self.state.overview.select(None),
            AppEvent::ToggleGroup => self.state.show_group ^= true,
            AppEvent::AddProject => {
                if !self.state.failed {
                    self.state.mode = Mode::Add;
                    self.state.project_form.state_mut().select_first();
                }
            }
            AppEvent::EditProject => {
                if !self.state.failed
                    && let Some(project) = self.selected_project().cloned()
                {
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
            AppEvent::LaunchProject => self.launch_selected(),
            AppEvent::EditSettings => {
                if let Err(err) = self.open_editor(ProjectHandler::edit_settings) {
                    self.state.mode = Mode::Error(err.to_string());
                }
            }
            AppEvent::EditScript => {
                if let Some(project) = self.selected_project().cloned()
                    && let Err(err) =
                        self.open_editor(|handler| handler.edit_project_script(&project))
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
        }
        Ok(())
    }

    fn handle_project_form_event(&mut self, event: AppEvent) -> color_eyre::Result<()> {
        match event {
            AppEvent::SelectNext => self.state.project_form.state_mut().select_next(),
            AppEvent::SelectPrevious => self.state.project_form.state_mut().select_previous(),
            AppEvent::MoveLeft => self.state.project_form.move_cursor_left(),
            AppEvent::MoveRight => self.state.project_form.move_cursor_right(),
            AppEvent::Char(ch) => self.state.project_form.put(ch),
            AppEvent::Backspace => self.state.project_form.backspace(),
            AppEvent::Submit => self.handle_form_submit()?,
            AppEvent::Abort => {
                self.state.project_form.state_mut().clear_all();
                self.state.mode = Mode::Home;
            }
            _ => self.state.needs_redraw = false,
        }
        Ok(())
    }

    fn handle_filter_event(&mut self, event: AppEvent) -> color_eyre::Result<()> {
        match event {
            AppEvent::MoveLeft => self.state.filter.move_cursor_left(),
            AppEvent::MoveRight => self.state.filter.move_cursor_right(),
            AppEvent::Char(ch) => self.state.filter.put(ch),
            AppEvent::Backspace => self.state.filter.backspace(),
            AppEvent::Submit => self.handle_filter_submit()?,
            AppEvent::Abort => self.go_home(),
            _ => self.state.needs_redraw = false,
        }
        Ok(())
    }

    fn handle_remove_event(&mut self, event: AppEvent) -> color_eyre::Result<()> {
        match event {
            AppEvent::Submit => {
                let Some(project) = self.selected_project().cloned() else {
                    return Ok(());
                };
                if let Err(err) = self.project_handler.remove_project(&project) {
                    self.state.mode = Mode::Error(err.to_string());
                    return Ok(());
                }
                self.project_handler.write_config()?;
                self.go_home();
            }
            AppEvent::Abort => self.state.mode = Mode::Home,
            _ => self.state.needs_redraw = false,
        }
        Ok(())
    }

    fn handle_error_event(&mut self, event: AppEvent) -> color_eyre::Result<()> {
        match event {
            AppEvent::Submit => {
                self.state.needs_reload = true;
                self.go_home();
            }
            _ => self.state.needs_redraw = false,
        }
        Ok(())
    }

    /// Launches the selected project in the background, surfacing launch failures.
    fn launch_selected(&mut self) {
        let Some(project) = self.selected_project().cloned() else {
            return;
        };
        self.set_status(
            format!("launching {}", project.name),
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        );
        let sender = self.events.sender();
        let result = self
            .project_handler
            .launch_project(&project, move |outcome| {
                let event = match outcome {
                    Ok(res) => {
                        AppEvent::launch_finished(res.success, res.code, res.stdout, res.stderr)
                    }
                    Err(error) => AppEvent::launch_failed(error),
                };
                let _ = sender.send(Event::App(event));
            });
        if let Err(err) = result {
            self.state.mode = Mode::Error(err.to_string());
        }
    }

    /// Shows a failure popup when a background launch reports an error.
    fn report_launch_result(
        &mut self,
        success: bool,
        code: Option<i32>,
        stdout: String,
        stderr: String,
    ) {
        if !success {
            let output = if stderr.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            let exit = code
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default();
            let red = ratatui::style::Style::default().fg(ratatui::style::Color::Red);
            self.set_status(format!("launch failed{exit}"), red);
            self.state.mode = Mode::Error(format!("Script failed{exit}\n{output}"));
        } else {
            let green = ratatui::style::Style::default().fg(ratatui::style::Color::Green);
            self.set_status("launch succeeded".to_owned(), green);
        }
    }

    const STATUS_TTL: std::time::Duration = std::time::Duration::from_secs(5);

    fn set_status(&mut self, message: String, style: ratatui::style::Style) {
        self.state.status = Some((message, style));
        self.state.status_generation += 1;
        let generation = self.state.status_generation;

        // Replace any pending expiration task so only the newest status's timer is live.
        if let Some(old) = self.status_task.take() {
            old.abort();
        }
        let ttl = std::env::var("PROJECTOR_STATUS_TTL_MS")
            .ok()
            .and_then(|raw| raw.parse().ok())
            .map(std::time::Duration::from_millis)
            .unwrap_or(Self::STATUS_TTL);
        let sender = self.events.sender();
        self.status_task = Some(tokio::spawn(async move {
            tokio::time::sleep(ttl).await;
            let _ = sender.send(Event::App(AppEvent::StatusExpired(generation)));
        }));

        self.state.needs_redraw = true;
    }

    /// Clears the status only if `generation` matches the one currently displayed.
    fn expire_status(&mut self, generation: usize) {
        if self.state.status_generation == generation {
            self.state.status = None;
            self.state.needs_redraw = true;
        }
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        if let Some(app_event) = self.map_key(&key_event) {
            self.events.send(app_event);
        }
        Ok(())
    }

    /// Maps a key press in the current mode to an application event, if any.
    fn map_key(&self, key_event: &KeyEvent) -> Option<AppEvent> {
        match self.state.mode {
            Mode::Home => self.map_home_key(key_event),
            Mode::Add | Mode::Edit => self.map_form_key(key_event, false),
            Mode::Filter => self.map_form_key(key_event, true),
            Mode::Remove => self.map_remove_key(key_event),
            Mode::Error(_) => self.map_error_key(key_event),
        }
    }

    fn map_home_key(&self, key_event: &KeyEvent) -> Option<AppEvent> {
        match key_event.code {
            KeyCode::Esc => Some(if self.state.filter.active {
                AppEvent::Abort
            } else {
                AppEvent::Quit
            }),
            KeyCode::Char(' ') | KeyCode::Enter => Some(AppEvent::LaunchProject),
            KeyCode::Char('g') => Some(AppEvent::SelectFirst),
            KeyCode::Char('G') => Some(AppEvent::SelectLast),
            KeyCode::Down | KeyCode::Char('j') => Some(AppEvent::SelectNext),
            KeyCode::Up | KeyCode::Char('k') => Some(AppEvent::SelectPrevious),
            KeyCode::Left | KeyCode::Char('h') => Some(AppEvent::Unselect),
            KeyCode::Right | KeyCode::Char('l') => Some(AppEvent::EditProject),
            KeyCode::Char('e') => Some(AppEvent::EditScript),
            KeyCode::Char('a' | 'n') => Some(AppEvent::AddProject),
            KeyCode::Char('d') => Some(AppEvent::RemoveProject),
            KeyCode::Char('i') => Some(AppEvent::ToggleGroup),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                Some(AppEvent::Quit)
            }
            KeyCode::Char('s') => Some(AppEvent::EditSettings),
            KeyCode::Char('f' | 'F' | '/') => Some(AppEvent::FilterProject),
            KeyCode::Char('r' | 'R') => Some(AppEvent::Reload),
            KeyCode::Char('q') => Some(AppEvent::Quit),
            _ => None,
        }
    }

    fn map_form_key(&self, key_event: &KeyEvent, is_filter: bool) -> Option<AppEvent> {
        match key_event.code {
            KeyCode::Esc => Some(AppEvent::Abort),
            KeyCode::Down | KeyCode::Tab => Some(if is_filter {
                AppEvent::Submit
            } else {
                AppEvent::SelectNext
            }),
            KeyCode::Up | KeyCode::BackTab => Some(AppEvent::SelectPrevious),
            KeyCode::Left => Some(AppEvent::MoveLeft),
            KeyCode::Right => Some(AppEvent::MoveRight),
            KeyCode::Char(ch) => Some(AppEvent::Char(ch)),
            KeyCode::Backspace => Some(AppEvent::Backspace),
            KeyCode::Enter => Some(AppEvent::Submit),
            _ => None,
        }
    }

    fn map_remove_key(&self, key_event: &KeyEvent) -> Option<AppEvent> {
        match key_event.code {
            KeyCode::Enter | KeyCode::Char('y' | 'Y') => Some(AppEvent::Submit),
            KeyCode::Esc | KeyCode::Char('n' | 'N') => Some(AppEvent::Abort),
            KeyCode::Char('q') => Some(AppEvent::Quit),
            _ => None,
        }
    }

    fn map_error_key(&self, key_event: &KeyEvent) -> Option<AppEvent> {
        match key_event.code {
            // `Enter` and `Space` are the "launch" hotkeys, so treating them as
            // "dismiss" risks dismissing an error the user just caused by
            // reflexively pressing them to launch the next project.
            KeyCode::Enter | KeyCode::Char(' ') => None,
            KeyCode::Char('q') => Some(AppEvent::Quit),
            _ => Some(AppEvent::Submit),
        }
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
            status: None,
            status_generation: 0,
            overview,
            project_form: ProjectForm::default(),
            filter: FilterState::default(),
        }
    }
}
