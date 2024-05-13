fn main() {
    println!(
        "cargo:rustc-env=CARGO_PKG_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );
}
