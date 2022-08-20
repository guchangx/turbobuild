use std::{io::Read, iter::FromIterator};

use tokio::time::error::Elapsed;
use winapi::um::winreg::RegOpenKeyExW;

extern crate axum;
extern crate winapi;


#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
struct CompileInfo {
    compiler_path: std::ffi::OsString,
    work_dir: std::ffi::OsString,
    compiler_args:  Vec<std::ffi::OsString>,
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

    let mut args =  compileInfo.compiler_args;
    startlocalcompiler(std::ffi::OsString::from(""), compileInfo.work_dir, 
                   compileInfo.compiler_path, &mut args);
    
    axum::extract::Json("{compile done}".to_string())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {

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

fn startlocalcompiler(key: std::ffi::OsString, workingdir: std::ffi::OsString, compilerpath: std::ffi::OsString, compilerargs:&mut  Vec<std::ffi::OsString>) -> bool {
    use std::process::{Stdio, ChildStdin, ChildStderr};

    let winkitslincludes = getwinsdkincludespath();

    match winkitslincludes {
        Some(includes) => {
            for mut include in includes {
                let mut instruct = "/I".to_string();
                instruct += &include;
                compilerargs.push(std::ffi::OsString::from(instruct));
            }
        },
        _ => {
            println!("do not get win kits includes");
        },
    }

    let includepath = getLocalCompileIncludeFilesPath();
    match includepath {
        Some(includefilepath) => {
            let mut instruct = "/I".to_string();
            instruct += &includefilepath;
            compilerargs.push(std::ffi::OsString::from(instruct));
        },
        _ => {
            println!("msvc include path do not find");
        },
    };

    let workpath = std::env::current_dir().unwrap();
    //println!("current exe path:{:?}, compile path: {:?}, do compile job path: {:?}", workpath.clone(), compilerpath, workingdir);

    // let result = std::process::Command::new(compilerpath)
    //                 .current_dir(workingdir)
    //                 .args(compilerargs.clone())
    //                 .stdout(Stdio::piped())
    //                 .stderr(Stdio::piped())
    //                 .output()
    //                 .expect("failed to execute compoiler process!");
    
    // if result.status.success() {
    //     let output = String::from_utf8_lossy(&result.stdout);
    //     for line in output.lines() {
    //         println!("{:#?}", line);
    //     }
    //     println!("build success!");
    //     return true;
    // }
    // else {
    //     let output = String::from_utf8_lossy(&result.stdout);
    //     let outputlines = output.lines();
    //     for line in outputlines {
    //         println!("{:#?}", line);
    //     }
    //     return false;
    // } 

    let exit_status = std::process::Command::new(compilerpath)
                            .current_dir(workingdir)
                            .args(compilerargs.clone())
                            .status();
        match exit_status {
            Ok(status) => {
                if status.success() {
                    return true
                }
                else {
                    return false
                }
            },
            Err(error) => {
                println!("do compile failed, error info: {:?}", error);
                return false;
            }
        }
}

fn getVSInstallPath() -> Option<String> {

    let programfilespath = std::env::var_os("PROGRAMFILES(X86)").unwrap();
    //"C:\Program Files (x86)"
    let vswhere = std::path::Path::new(programfilespath.to_str().unwrap()).join("Microsoft Visual Studio")
    .join("Installer").join("vswhere.exe");

    let is_exist = vswhere.exists();
    if is_exist {
        //println!("vs where is exits:{:?}", vswhere); 
            
        let result =  std::process::Command::new(vswhere)
        .arg("-latest")
        .arg("-products").arg("*")
        .arg("-requires").arg("Microsoft.VisualStudio.Component.VC.Tools.x86.x64")
        .arg("-property").arg("installationPath")
        .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    let mut vsinstallpath = String::from_utf8_lossy(&output.stdout);
                    let purevsinstallpath = vsinstallpath.replace("\r\n", "");
                    return Some(purevsinstallpath)
                }
                else {
                    println!("execute vswhere.exe result error: {:?}", output.status.code());
                    return None
                }
            },
            Err(error) => {
                println!("execute vswhere.exe failed: {:?}", error);
                return None
            },
        }
    } else {
        println!("vs where is not exits: {:?}", vswhere); 
        return None
    }

}

fn getLocalCompileIncludeFilesPath() ->Option<String> {

    let vsinstallpath = getVSInstallPath();
    match vsinstallpath {
        None => {
            println!("get vs install path failed");
        },
        Some(vspath) => {
            //C:\Program Files (x86)\Microsoft Visual Studio\2019\Enterprise\VC\Auxiliary\Build
            let vcauxiliarybuildpath = std::path::Path::new(vspath.as_str()).join("VC").join("Auxiliary").join("Build")
            .join("Microsoft.VCToolsVersion.default.txt");
            if vcauxiliarybuildpath.exists() {
                let versionfile = std::fs::File::open(vcauxiliarybuildpath);
                match versionfile {
                    Ok(mut file) => {

                        let mut vctoolsversion = String::new();
                        let _version = file.read_to_string(&mut vctoolsversion);
                        if vctoolsversion.len() > 2 
                        {
                            let purevctoolsversion = vctoolsversion.replace("\r\n", "");
                            let msvcincludepath = std::path::Path::new(vspath.as_str()).join("vc").join("Tools").join("MSVC")
                                    .join(purevctoolsversion.as_str()).join("include");
                            
                            if msvcincludepath.exists() {
                                //println!("msvc include path: {:#?}", msvcincludepath);
                                return Some(msvcincludepath.into_os_string().into_string().unwrap())
                            }
                            else {
                                println!("do not open vc auxiliary build path: {:?}", msvcincludepath);
                            }
                            
                        };

                    },
                    Err(error) => {
                        println!("do not open vc auxiliary build path: {:?}", error);
                        return None;
                    },
                };
            } else {
                println!("vc tools version default do not exist: {:?}", vcauxiliarybuildpath);
                return None;
            }
        },
    };
    return None; 
}


fn getwinsdkincludespath() -> Option<Vec<String>> {
    use std::os::windows::ffi::OsStrExt;
    use std::iter::once;
    use std::ptr::null_mut;
    unsafe {

        //HKEY_LOCAL_MACHINE\SOFTWARE\Wow6432Node\Microsoft\Microsoft SDKs\Windows\v10
        let mut subKey: Vec<u16> = std::ffi::OsStr::new(r"SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0").encode_wide().chain(once(0)).collect();
        let mut phkResult: winapi::shared::minwindef::HKEY = null_mut();
        let openstatus = winapi::um::winreg::RegOpenKeyW(winapi::um::winreg::HKEY_LOCAL_MACHINE, 
            subKey.as_ptr(), &mut phkResult);
        
        if openstatus == winapi::shared::winerror::SEC_E_OK {

            let valuename: Vec<u16> = std::ffi::OsStr::new("InstallationFolder").encode_wide().chain(once(0)).collect();

            let mut dword: winapi::shared::minwindef::DWORD = 128;
            let mut data = vec![0; 128 as usize];

            let querystatus = winapi::um::winreg::RegQueryValueExW(phkResult, valuename.as_ptr(), null_mut(), 
              &mut winapi::um::winnt::REG_SZ, data.as_mut_ptr(), &mut dword);

            let mut winkits_path = String::new();
            if querystatus == winapi::shared::winerror::SEC_E_OK {

                data.set_len(dword as usize);
                let words = std::slice::from_raw_parts(data.as_ptr() as *const u16, data.len() / 2);
                winkits_path = String::from_utf16_lossy(words);

                winkits_path = winkits_path.trim_end_matches('\0').to_string();
            }
            else {
                println!("get regedit InstallationFolder failed . error code: {:?}", querystatus);
            }
            let product_version: Vec<u16> = std::ffi::OsStr::new("ProductVersion").encode_wide().chain(once(0)).collect();

            let mut version_data = vec![0; 32];
            let mut version_len : winapi::shared::minwindef::DWORD = 32;
            let query_version_status = winapi::um::winreg::RegQueryValueExW(phkResult, product_version.as_ptr(), null_mut(),
             &mut winapi::um::winnt::REG_SZ, version_data.as_mut_ptr(),  &mut version_len);

            let mut winkits_version = String::new();
            if query_version_status == winapi::shared::winerror::SEC_E_OK {

                version_data.set_len(version_len as usize);
                let words = std::slice::from_raw_parts(version_data.as_ptr() as *const u16, version_data.len() / 2);
                winkits_version = String::from_utf16_lossy(words);
                winkits_version = winkits_version.trim_end_matches('\0').to_string();
                winkits_version.push_str(".0");

            }
            else {
                println!("get regedit sdks version failed . error code: {:?}", query_version_status);
            }

            if !winkits_path.is_empty() && !winkits_version.is_empty()
            {
                let includepath = std::path::Path::new(winkits_path.as_str()).join("Include").join(winkits_version.as_str());
               
                let mut includespath : Vec<String> = Vec::new();
                let cppwinrt_include = includepath.join("cppwinrt");

                if cppwinrt_include.exists() {
                    includespath.push(cppwinrt_include.into_os_string().into_string().unwrap());
                }

                let shared_include = includepath.join("shared");
                if shared_include.exists() {
                    includespath.push(shared_include.into_os_string().into_string().unwrap());
                }

                let ucrt_include = includepath.join("ucrt");
                if ucrt_include.exists() {
                    includespath.push(ucrt_include.into_os_string().into_string().unwrap());
                }

                let um_include = includepath.join("um");
                if um_include.exists() {
                    includespath.push(um_include.clone().into_os_string().into_string().unwrap());
                }

                if um_include.join("winsdkver.h").exists() || um_include.join("windows.h").exists() {
                    //println!("winsdkver.h and windows.h files exists");
                }

                let winrt_include = includepath.join("winrt");
                if winrt_include.exists() {
                    includespath.push(winrt_include.into_os_string().into_string().unwrap());
                }
                return Some(includespath)
            }
             
        } else {
            println!("open regedit failed, error: {:?}.", openstatus);
        }
        return None
    }
}