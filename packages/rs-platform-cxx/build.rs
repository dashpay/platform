fn main() {
    cxx_build::CFG.include_prefix = "dash/platform";
    cxx_build::bridge("src/lib.rs")
        .std("c++20")
        .compile("dash-platform-cxx-bridge");

    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=signer.h");
}
