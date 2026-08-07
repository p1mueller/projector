use crate::app::App;

pub mod app;
pub mod event;
pub mod forms;
pub mod project;
pub mod ui;
pub mod widgets;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let result = App::new()?.run().await;
    ratatui::restore();
    result
}
