
fn main()  {
    println!("cocrew crate build xxxxx.proto file.");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=./../proto/pack.proto");
    
    std::env::set_var("PROTOC", "./../vendor/protoc-28.1-win64/bin/protoc.exe");
    
    let include = [
        "./../proto",
    ];
    
    let res = tonic_prost_build::configure()
    .extern_path(".prost", "::prost")
    .build_server(true)
    .build_client(false)
    .out_dir("./proto")
    //.type_attribute(".", "#[derive(serde_derive::Deserialize, serde_derive::Serialize)]")
    .compile_protos(
        &["./../proto/pack.proto"],
        &include,
    );
    
    match res {
        Ok(_) => println!("cocrew compile pack proto file success."),
        Err(e) => println!("cocrew compile pack proto file error: {}", e),
    }

    if std::path::Path::new("./3dparty/detours/lib.X64/detours.lib").exists()
    {
        println!("cargo:note=cocrew 3dparty detours found.");
        println!("cargo:rerun-if-changed=./3dparty/detours/include/detours.h");
        println!("cargo:rustc-link-search=native=./cocrew/3dparty/detours/lib.X64");
        println!("cargo:rustc-link-lib=static=detours");
    }
    else {
        println!("cargo:warning=cocrew 3dparty detours not found.");
    }

}
