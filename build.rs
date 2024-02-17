use copy_to_output::copy_to_output;
use std::env;
extern crate embed_resource;

fn main() {
    embed_resource::compile("resources/res.rc", embed_resource::NONE);
    copy_to_output("default.yaml", &env::var("PROFILE").unwrap()).expect("Could not copy");
    copy_to_output("user.yaml", &env::var("PROFILE").unwrap()).expect("Could not copy");
    copy_to_output("README.md", &env::var("PROFILE").unwrap()).expect("Could not copy");
}
