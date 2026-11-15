use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let enet = root.join("../../vendor/enet");
    let include = enet.join("include");

    let mut build = cc::Build::new();
    build.include(&include);
    build.warnings(false);

    for file in [
        "callbacks.c",
        "compress.c",
        "host.c",
        "list.c",
        "packet.c",
        "peer.c",
        "protocol.c",
    ] {
        build.file(enet.join(file));
    }

    if cfg!(windows) {
        build.file(enet.join("win32.c"));
    } else {
        build.file(enet.join("unix.c"));
    }

    build.file(root.join("src/shim.c"));
    build.compile("gtps_enet");

    println!("cargo:rerun-if-changed={}", enet.display());
    println!("cargo:rerun-if-changed={}", include.display());
    println!("cargo:rerun-if-changed=src/shim.c");
    println!("cargo:include={}", include.display());

    if !cfg!(windows) {
        println!("cargo:rustc-link-lib=m");
    }
}
