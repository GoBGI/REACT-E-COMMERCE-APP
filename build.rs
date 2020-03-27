extern crate cc;

fn main() {
    cc::Build::new()
        .flag("-std=c99")
        .flag("-Wall")
        .flag("-Wextra")
        .flag("-pedantic")
        .file("src/audio