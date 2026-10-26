fn main() {
    kai_build::app()
        .generate()
        .unwrap_or_else(|error| panic!("{error}"));
}
