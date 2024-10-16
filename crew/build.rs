
fn main()  {
    println!("crew crate build xxxxx.proto file.");
    println!("cargo:rerun-if-changed=build.rs");
    
    std::env::set_var("PROTOC", "./../vendor/protoc-28.1-win64/bin/protoc.exe");
    
    let include = [
        "./../proto",
    ];
    
    let res = tonic_build::configure()
        .extern_path(".prost", "::prost")
        .build_server(false)
        .build_client(true)
        .out_dir("./proto")
        .compile_protos(
            &["./../proto/notify.proto", "./../proto/pack.proto"],
            &include,
        );
    
    match res {
        Ok(_) => println!("crew compile notify proto file success."),
        Err(e) => println!("crew compile notify proto file error: {}", e),
    }
}