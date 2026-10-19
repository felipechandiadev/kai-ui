//! Control gallery: `cargo run --example gallery [-- --theme kai|appkit
//! --appearance light|dark --inactive --section buttons]`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gallery = kai::gallery::Gallery::from_args(std::env::args().skip(1));
    let theme = gallery.current_theme();
    kai::runtime::run(gallery, theme)
}
