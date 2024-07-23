
fn main() {

    println!("cargo:rerun-if-changed=build.rs");
    
    println!("cargo:rerun-if-changed=./3dparty/detours/include/detours.h");

    println!("cargo:rustc-link-search=native=./turbobuild/3dparty/detours/lib.X64");
    println!("cargo:rustc-link-lib=static=detours");
}