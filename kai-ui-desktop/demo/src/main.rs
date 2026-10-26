use kai::theme::{Density, Theme};
use kai_demo::{app, state::State};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run(State::default(), Theme::kai(Density::Comfortable))
}
