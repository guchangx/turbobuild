
fn main() {

    println!("cargo:rerun-if-changed=build.rs");
    
    if std::path::Path::new("./../cocrew/3dparty/detours/lib.X64/detours.lib").exists()
    {
        println!("cargo:note=redirect cocrew 3dparty detours found.");
        println!("cargo:rerun-if-changed=./3dparty/detours/include/detours.h");
        println!("cargo:rustc-link-search=native=./cocrew/3dparty/detours/lib.X64");
        println!("cargo:rustc-link-lib=static=detours");
    }
    else
    {
        println!("cargo:warning=redirect cocrew 3dparty detours not found.")
    }
}