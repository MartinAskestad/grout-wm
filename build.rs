use copy_to_output::copy_to_output;
use std::env;

fn main() {
    copy_to_output("default.yaml", &env::var("PROFILE").unwrap()).expect("Could not copy");
    copy_to_output("default.toml", &env::var("PROFILE").unwrap()).expect("Could not copy");
}
