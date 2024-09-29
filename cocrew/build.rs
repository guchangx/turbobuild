
fn main()  {
    println!("cocrew crate build xxxxx.proto file.");

    std::env::set_var("PROTOC", "./../vendor/protoc-28.1-win64/bin/protoc.exe");
    
    let include = [
        "./../proto",
    ];
    
    let res = tonic_build::configure()
    .extern_path(".prost", "::prost")
    .build_server(true)
    .build_client(false)
    .out_dir("./proto")
    //.type_attribute(".", "#[derive(serde_derive::Deserialize, serde_derive::Serialize)]")
    .compile(
        &["./../proto/pack.proto"],
        &include,
    );
    
    match res {
        Ok(_) => println!("cocrew compile pack proto file success."),
        Err(e) => println!("cocrew compile pack proto file error: {}", e),
    }
}
