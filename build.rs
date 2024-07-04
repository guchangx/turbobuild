
use std::env;
fn main() {
    env::set_var("LIBCLANG_PATH", "./tool/clang/Windows/x64/bin");

    println!("cargo:rerun-if-changed=build.rs");
    
    println!("cargo:rerun-if-changed=./3dparty/detours/include/detours.h");

    println!("cargo:rustc-link-search=native=./3dparty/detours/lib.X64");
    println!("cargo:rustc-link-lib=static=detours");

    println!("cargo:rustc-env=LIBCLANG_PATH='./tool/clang/libWindows/x64/bin'");

    let bindgen = bindgen::Builder::default()
        //.clang_arg("_AMD64_")
        .clang_arg("-v")
        .clang_arg("-fms-compatibility")
        .clang_arg("-fms-extensions")
        .clang_args(&["-I", "3dparty/detours/include"])
        //.clang_args(&["-I", "<Windows.h>"])
        //.header("3dparty/detours/include/detours.h")
        .header_contents(
            "bindgen.h",
            r#"
                #include <Windows.h>
                #include "detours.h"
            "#
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .whitelist_function("Detour.*")
        .generate()
        .expect("Unable to generate bindings");

    bindgen.write_to_file("src/detours.rs")
        .expect("can not write bindings");
    
}