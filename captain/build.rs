
fn main()  {
    println!("captain build xxxxx.proto file.");

    std::env::set_var("PROTOC", "./../vendor/protoc-28.1-win64/bin/protoc.exe");
    
    let include = [
        "./../proto",
    ];
    
    let res = tonic_prost_build::configure()
    .extern_path(".prost", "::prost")
    .build_server(true)
    .out_dir("./proto")
    //.type_attribute(".", "#[derive(serde_derive::Deserialize, serde_derive::Serialize)]")
    .compile_protos(
        &["./../proto/notify.proto"],
        &include,
    );
    
    match res {
        Ok(_) => println!("captain compile proto file success."),
        Err(e) => println!("captain compile proto file error: {}", e),
    }
}