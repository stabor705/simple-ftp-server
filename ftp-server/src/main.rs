mod app;
mod config;

use app::App;

fn main() {
    if let Err(err) = App::run() {
        eprint!("{}", err);
    }
}
