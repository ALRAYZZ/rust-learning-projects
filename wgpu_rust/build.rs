// When running cargo run or cargo build, cargo does a first check to find
// build.rs on the root of the project. If it exists, cargo compiles and runs it
// before compiling the rest of the project. This is useful for tasks like
// code generation, compiling native dependencies, or in our case, copying
// resource files to the output directory so they are available at runtime.

use anyhow::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;



fn main() -> Result<()> {
    // Tells cargo to rerun this build script if anything in res/ changes
    println!("cargo::rerun-if-changed=res/*");

    // OUT_DIR is an environment variable Cargo uses to specify where app is built
    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("res/");
    copy_items(&paths_to_copy, &out_dir, &copy_options)?;

    Ok(())
}