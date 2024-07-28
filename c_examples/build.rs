fn main() {
    let config = cbindgen::Config::from_file("../cbindgen.toml").unwrap();
    cbindgen::Builder::new()
        .with_crate("..")
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("target/include/libtypec-rs.h");

    cc::Build::new()
        .file("lstypec.c")
        .include("target/include")
        .compile("c_examples_lstypec");

    println!("cargo::rerun-if-changed=lstypec.c");
}
