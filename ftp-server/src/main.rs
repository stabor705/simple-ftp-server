mod app;
mod config;

use app::App;
pub use config::{Config, TomlConfig};

fn main() {
    if let Err(err) = App::run() {
        eprint!("{}", err);
    }
}
