use crate::app::App;

pub mod app;
pub mod cli;
pub mod client;
pub mod error;
pub mod pda;

pub fn main() {
    let app = App::init();
    app.run();
}
