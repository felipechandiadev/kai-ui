use kai::theme::{Density, Theme};
use kai_sections_app::{app, state};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run(state::State::sample(), Theme::kai(Density::Comfortable))
}
