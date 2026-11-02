use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use prost::Message;
use prost_types::FileDescriptorSet;

#[path = "build/bnet_routes.rs"]
mod bnet_routes;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR is missing")?);

    let schemas_path = manifest_dir.join("src/protocol/bnet/bnet.protoset");

    println!("cargo:rerun-if-changed={}", schemas_path.display(),);
    println!("cargo:rerun-if-changed=build.rs");

    let bytes = fs::read(&schemas_path)?;
    let mut schemas = FileDescriptorSet::decode(bytes.as_slice())?;

    for file in &mut schemas.file {
        file.source_code_info.get_or_insert_with(Default::default);
    }

    let mut config = prost_build::Config::new();

    config
        .disable_comments(["."])
        .enable_type_names()
        .include_file("bnet_generated.rs");

    println!("cargo:rerun-if-changed=build/bnet_routes.rs");
    bnet_routes::generate(&bytes, &schemas)?;

    config.compile_fds(schemas)?;

    Ok(())
}
