//! Terminal event loop.
//!
//! [`EventHandler`] owns the ratatui terminal and a channel of [`Event`]s
//! (terminal- or app-level events, produced by a background event-reader task),
//! and drives the TUI: enter/exit the alternate screen, receive events, and
//! forward [`AppEvent`]s to the running application.
//!
//! [`AppEvent`] is the set of high-level actions the app can perform (add /
//! edit / remove a project, navigate, launch, etc.).

use color_eyre::eyre::{OptionExt, eyre};
use crossterm::{
    cursor,
    event::Event as CrosstermEvent,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{FutureExt, StreamExt};
use ratatui::backend::CrosstermBackend as Backend;
use std::{
    io::{Stdout, stdout},
    ops::{Deref, DerefMut},
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;

/// Representation of all possible events.
#[derive(Clone, Debug)]
pub enum Event {
    /// Crossterm events.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Application events.
    ///
    /// Use this event to emit custom events that are specific to your application.
    App(AppEvent),
}

/// Application events.
///
/// You can extend this enum with your own custom events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    /// Begin the *Add project* flow.
    AddProject,
    /// Begin the *Edit project* flow for the selected project.
    EditProject,
    /// Ask for confirmation to remove the selected project.
    RemoveProject,
    /// Enter the *Filter* flow.
    FilterProject,
    /// Open the settings file in the editor.
    EditSettings,
    /// Open the selected project's script in the editor.
    EditScript,
    /// Launch the selected project in the background.
    LaunchProject,
    /// Toggle whether the overview shows parent groups.
    ToggleGroup,
    /// Cycle to the next sort mode and re-sort the list.
    ToggleSort,
    /// A character typed into the focused input.
    Char(char),
    /// Paste the system clipboard into the focused input.
    Paste,
    /// Backspace in the focused input.
    Backspace,
    /// Submit the current form / filter.
    Submit,
    /// Result of finishing a project launch (spawn-time failure included).
    LaunchFinished {
        /// Whether the script exited successfully.
        success: bool,
        /// The script's exit code, if it exited normally.
        code: Option<i32>,
        /// The script's standard output.
        stdout: String,
        /// The script's standard error.
        stderr: String,
    },
    /// Cancel the current flow without changing state.
    Abort,
    /// Move the input caret one character left.
    MoveLeft,
    /// Move the input caret one character right.
    MoveRight,
    /// Select the first item in the list.
    SelectFirst,
    /// Select the last item in the list.
    SelectLast,
    /// Select the next item in the list.
    SelectNext,
    /// Select the previous item in the list.
    SelectPrevious,
    /// Clear the list selection.
    Unselect,
    /// Mark the config for a reload on the next loop iteration.
    Reload,
    /// Fired by the status TTL task when it lapses; carries the generation it was built for.
    StatusExpired(usize),
    /// Quit the application.
    Quit,
}

impl AppEvent {
    /// Builds a [`AppEvent::LaunchFinished`] from a launch outcome.
    pub fn launch_finished(
        success: bool,
        code: Option<i32>,
        stdout: String,
        stderr: String,
    ) -> Self {
        Self::LaunchFinished {
            success,
            code,
            stdout,
            stderr,
        }
    }

    /// Builds a [`AppEvent::LaunchFinished`] that reports a spawn-time error.
    pub fn launch_failed(error: String) -> Self {
        Self::LaunchFinished {
            success: false,
            code: None,
            stdout: String::new(),
            stderr: error,
        }
    }
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    terminal: ratatui::Terminal<Backend<Stdout>>,
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver channel.
    receiver: mpsc::UnboundedReceiver<Event>,
    /// Handle to the event-reader task started by [`EventHandler::enter`].
    task: JoinHandle<()>,
    /// Token used to stop the event-reader task.
    cancellation_token: CancellationToken,
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns a new thread to handle events.
    pub fn new() -> color_eyre::Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        // let actor = EventThread::new(sender.clone());
        // let task = tokio::spawn(async { actor.run().await });
        let task = tokio::spawn(async {});
        let terminal = ratatui::Terminal::new(Backend::new(stdout()))?;
        Ok(Self {
            terminal,
            sender,
            receiver,
            task,
            cancellation_token: CancellationToken::new(),
        })
    }

    /// Enter the TUI: enable raw mode, switch to the alternate screen,
    /// hide the cursor, and (re)start the event-reader task.
    pub fn enter(&mut self) -> color_eyre::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let actor = EventThread::new(self.sender.clone(), self.cancellation_token.clone());
        self.task = tokio::spawn(async {
            actor.run().await;
        });
        Ok(())
    }

    /// Leave the TUI: stop the event task, then restore raw mode and the
    /// cursor if still active.
    pub fn exit(&mut self) -> color_eyre::Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            crossterm::execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    /// Cancel the event-reader task and wait (up to ~100 ms) for it to finish.
    ///
    /// # Errors
    /// - Fails if the task cannot be joined within the timeout.
    pub fn stop(&self) -> color_eyre::Result<()> {
        self.cancel();
        let mut counter = 0;
        while !self.task.is_finished() {
            std::thread::sleep(Duration::from_millis(1));
            counter += 1;
            if counter > 50 {
                self.task.abort();
            }
            if counter > 100 {
                return Err(eyre!(
                    "Failed to abort task in 100 milliseconds for unknown reasons"
                ));
            }
        }
        Ok(())
    }

    /// Signal the event-reader task to shut down.
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Returns a clone of the sender, so background tasks can push events into the loop without
    /// holding a `&mut self`.
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }

    /// Queue an app event to be sent to the event receiver.
    ///
    /// This is useful for sending events to the event handler which will be processed by the next
    /// iteration of the application's event loop.
    pub fn send(&mut self, app_event: AppEvent) {
        // Ignore the result as the reciever cannot be dropped while this struct still has a
        // reference to it
        let _ = self.sender.send(Event::App(app_event));
    }
}

/// A thread that handles reading crossterm events.
struct EventThread {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
    cancellation_token: CancellationToken,
}

impl EventThread {
    /// Constructs a new instance of [`EventThread`].
    fn new(sender: mpsc::UnboundedSender<Event>, cancellation_token: CancellationToken) -> Self {
        Self {
            sender,
            cancellation_token,
        }
    }

    /// Runs the event thread.
    ///
    /// This function polls for crossterm events in between.
    async fn run(self) {
        let mut reader = crossterm::event::EventStream::new();
        loop {
            let crossterm_event = reader.next().fuse();
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    break;
                }
                _ = self.sender.closed() => {
                    break;
                }
                Some(Ok(evt)) = crossterm_event => {
                    self.send(Event::Crossterm(evt));
                }
            };
        }
        self.cancellation_token.cancel();
    }

    /// Sends an event to the receiver.
    fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}

impl Deref for EventHandler {
    type Target = ratatui::Terminal<Backend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for EventHandler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        self.exit().unwrap();
    }
}
