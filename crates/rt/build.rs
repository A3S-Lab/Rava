fn main() {
    cc::Build::new()
        .file("src/lib.c")
        .opt_level(2)
        .compile("rava_rt");
    println!("cargo:rerun-if-changed=src/lib.c");
}
