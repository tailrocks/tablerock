fn main() {
    println!("cargo:rerun-if-env-changed=TABLEROCK_VERSION_OVERRIDE");
    let version = std::env::var("TABLEROCK_VERSION_OVERRIDE").unwrap_or_else(|_| {
        std::env::var("CARGO_PKG_VERSION").expect("Cargo sets package version")
    });
    println!("cargo:rustc-env=TABLEROCK_VERSION={version}");
}
