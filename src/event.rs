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
    AddProject,
    EditProject,
    RemoveProject,
    FilterProject,
    EditSettings,
    EditScript,
    LaunchProject,
    ToggleGroup,
    Char(char),
    Backspace,
    Submit,
    LaunchFinished {
        success: bool,
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    Abort,
    MoveLeft,
    MoveRight,
    SelectFirst,
    SelectLast,
    SelectNext,
    SelectPrevious,
    Unselect,
    Reload,
    Quit,
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    terminal: ratatui::Terminal<Backend<Stdout>>,
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver channel.
    receiver: mpsc::UnboundedReceiver<Event>,
    task: JoinHandle<()>,
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

    pub fn exit(&mut self) -> color_eyre::Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            crossterm::execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

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
