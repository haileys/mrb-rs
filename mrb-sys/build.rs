fn main() {
    cc::Build::new()
        .file("src/wrapper.c")
        .compile("mruby_rust");

    println!("cargo:rustc-link-lib=mruby");
    println!("cargo:rustc-link-lib=mruby_rust");
}
