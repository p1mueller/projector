use projector::app::App;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let result = App::new()?.run().await;
    ratatui::restore();
    result
}
