use tokio::time::error::Elapsed;

extern crate axum;

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug)]
struct CompileInfo {
    compiler_path: String,
    work_dir: String,
    compiler_args: Vec<String>,
}

async fn get_hello_info() -> &'static str
{
    "hello, welcome use build turbo."
}

async fn get_teamworker_info() -> &'static str
{
    "get team worker info"
}

async fn do_build() -> &'static str
{
    "do build"
}

async fn test_json() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({"data": 123}))
}

struct CompileRequest {
    workpath: String,
    compilerpath: String,
    compilerargs: Vec<String>,
}

async fn respone_msvc_compile(axum::extract::Json(compileInfo) : axum::extract::Json<CompileInfo>) -> axum::extract::Json<String> {

    
    println!("compiler path {:?}", compileInfo.compiler_path);
    println!("compiler work path {:?}", compileInfo.work_dir);
    println!("compile args: {:?}",  compileInfo.compiler_args);

    let mut args =  compileInfo.compiler_args;
    startlocalcompiler(String::from(""), compileInfo.work_dir, 
                    compileInfo.compiler_path, &mut args);
    
    axum::extract::Json("{compile done}".to_string())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    getLocalCompileIncludeFilesPath();

    let router = axum::Router::new()
            .route("/hello", axum::routing::get(get_hello_info))
            .route("/teamworker", axum::routing::get(get_teamworker_info))
            .route("/dobuild", axum::routing::post(do_build))
            .route("/testjson", axum::routing::get(test_json).post(test_json))
            .route("/msvc/requestcompile", axum::routing::post(respone_msvc_compile))
            ;

    let addr = &"127.0.0.1:9302".parse().unwrap();
    let server = axum::Server::bind(addr)
        .serve(router.into_make_service());
        
    if let Err(err) = server.await {
        println!("server error: {}", err);
    }  
}

fn startlocalcompiler(key: String, workingdir: String, compilerpath: String, compilerargs:&mut Vec<String>) -> bool{
    use std::process::{Stdio};
    let compilerfilepath = compilerargs.last().unwrap();
    println!("compiler file path: {:?}", compilerfilepath);

    let workpath = std::env::current_dir().unwrap();
    println!("current exe path:{:?}", workpath.clone());

    let result = std::process::Command::new(compilerpath)
                    .current_dir(workingdir)
                    .args(compilerargs.clone())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .expect("failed to execute compoiler process!");

    if result.status.success() {

        println!("build success!");
        return true;
    }
    else {
        let output = String::from_utf8_lossy(&result.stdout);
        let outputlines = output.lines();
        println!("file compiler error:");
        for line in outputlines {
            println!("{:#?}", line);
        }
        return false;
    }           
}

fn getVSInstallPath() -> Option<String> {

    let programfilespath = std::env::var_os("PROGRAMFILES(X86)").unwrap();
    //"C:\Program Files (x86)"
    let vswhere = std::path::Path::new(programfilespath.to_str().unwrap()).join("Microsoft Visual Studio")
    .join("Installer").join("vswhere.exe");

    let is_exist = vswhere.exists();
    if is_exist {
        println!("vs where is exits:{:?}", vswhere); 

    } else {
        println!("vs where is not exits:{:?}", vswhere); 
        return None
    }
    
    let result =  std::process::Command::new(vswhere)
    .arg("-latest")
    .arg("-products").arg("*")
    .arg("-requires").arg("Microsoft.VisualStudio.Component.VC.Tools.x86.x64")
    .arg("-property").arg("installationPath")
    .output()
    .expect("failed to execute vswhere.exe");

    if result.status.success() {
        let vsinstallpath = String::from_utf8_lossy(&result.stdout);
        return Some(vsinstallpath.to_string())
    }
    else {
        println!("execute vswhere.exe failed");
        return None
    }
}

fn getLocalCompileIncludeFilesPath() -> String {

    let vsinstallpath = getVSInstallPath();
    match vsinstallpath {
        None => {
            println!("execute vswhere.exe failed");
        },
        Some(vspath) => {
            println!("execute vswhere.exe failed");
            let includepath = std::path::Path::new(vspath.as_str());
        },
    };
    "".to_string()
}