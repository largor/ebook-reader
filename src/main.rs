mod app;
mod epub_reader;
mod ui;
mod progress;
mod toc;

use anyhow::Result;
use app::App;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let file_path = args.get(1).map(|s| s.as_str());
    let mut app = App::new(file_path)?;
    app.run()
}
