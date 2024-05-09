use copy_to_output::copy_to_output;
use std::env;
extern crate embed_resource;
use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    embed_resource::compile("resources/res.rc", embed_resource::NONE);
    copy_to_output("default.yaml", &env::var("PROFILE").unwrap()).expect("Could not copy");
    copy_to_output("user.yaml", &env::var("PROFILE").unwrap()).expect("Could not copy");
    copy_to_output("README.md", &env::var("PROFILE").unwrap()).expect("Could not copy");

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut manifest = new_manifest("grout-wm");
        manifest = manifest.version(0, 1, 0, 0);
        embed_manifest(manifest).expect("Could not embed manifest");
    }
}
