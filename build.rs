use dbc_codegen::{Config, FeatureConfig};

fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    #[cfg(feature = "defmt")]
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    let dbc_path = "STM_BUS.dbc";
    let dbc_file = std::fs::read(dbc_path).unwrap();
    println!("cargo:rerun-if-changed={}", dbc_path);

    let config = Config::builder()
        .dbc_name(dbc_path)
        .dbc_content(&dbc_file)
        .allow_dead_code(true)
        .check_ranges(FeatureConfig::Always)
        .build();

    let mut out = std::io::BufWriter::new(std::fs::File::create("src/messages.rs").unwrap());
    dbc_codegen::codegen(config, &mut out).expect("dbc-codegen failed");
}
