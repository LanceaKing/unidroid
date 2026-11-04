fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Match Rust's static pthread linkage instead of Unicorn's dynamic linkage.
    if std::env::var("CARGO_CFG_TARGET_OS")? == "windows"
        && std::env::var("CARGO_CFG_TARGET_ENV")? == "gnu"
    {
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::write(out.join("libpthread.dll.a"), "INPUT ( libpthread.a )\n")?;
        println!("cargo:rustc-link-search=native={}", out.display());
    }
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
