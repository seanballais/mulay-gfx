use std::path::Path;

extern crate copy_to_output;
use copy_to_output::copy_to_output;

fn main() {
    let debug_build_path = Path::new("target/debug");
    let release_build_path = Path::new("target/release");

    if debug_build_path.exists() {
        copy_to_output("assets", "debug")
            .expect("Unable to copy to assets folder to debug build folder");
    }

    if release_build_path.exists() {
        copy_to_output("assets", "release")
            .expect("Unable to copy to assets folder to release build folder");
    }
}
