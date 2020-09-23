use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/wrapper.h");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    pkg_config::probe_library("bcm_host").unwrap();

    let preproc_header_path = out_path.join("header.h");
    let preprocessed = cc::Build::new().file("src/wrapper.h").expand();
    let mut preproc_header = File::create(&preproc_header_path).unwrap();
    preproc_header.write_all(&preprocessed).unwrap();

    let bindings = bindgen::Builder::default()
        .header(preproc_header_path.display().to_string())
        .blacklist_function("q.cvt(_r)?")
        .blacklist_function("strtold")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .rustfmt_bindings(true)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
