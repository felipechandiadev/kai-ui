//! Kai UI Desktop demo (`cargo run`); the demo lives in `kai::demo`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    kai::demo::run_from_env()
}
