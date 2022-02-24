use crate::app::App;

pub mod app;
pub mod cli;
pub mod error;

pub fn main() {
    let app = App::init();
    app.run();
}
