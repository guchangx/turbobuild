
fn main()  {
    println!("crew crate build xxxxx.proto file.");

    std::env::set_var("PROTOC", "./../vendor/protoc-28.1-win64/bin/protoc.exe");
    
    let include = [
        "./../proto",
    ];
    
    let res = tonic_build::configure()
        .extern_path(".prost", "::prost")
        .build_server(false)
        .build_client(true)
        .out_dir("./proto")
        .compile(
            &["./../proto/notify.proto"],
            &include,
        );
    
    match res {
        Ok(_) => println!("crew compile notify proto file success."),
        Err(e) => println!("crew compile notify proto file error: {}", e),
    }

    let res = tonic_build::configure()
        .extern_path(".prost", "::prost")
        .build_server(false)
        .build_client(true)
        .out_dir("./proto")
        .compile(
            &["./../proto/pack.proto"],
            &include,
        );  
    
    match res {
        Ok(_) => println!("crew compile package proto file success."),
        Err(e) => println!("crew compile package proto file error: {}", e),
    }
}