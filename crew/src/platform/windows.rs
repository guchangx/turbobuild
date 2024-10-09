extern crate winapi;

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct WindowsCompilerEnv {
    pub winkits_includes_path: Vec<String>,
    pub compiler_path: std::path::PathBuf,
    pub msvc_includes_path: std::path::PathBuf,
    pub msvc_version: String,
    pub env_args: String,
}

impl Default for WindowsCompilerEnv {
    fn default() -> Self {
        let winsdk_includes = get_winsdk_includes_path().unwrap();
        let local_msvc_includes = get_local_msvc_include_files_path().unwrap();
        let local_msvc_install = get_local_msvc_bin_path().unwrap();
        let msvc_version = get_msvc_version().unwrap();
        Self { 
            winkits_includes_path: winsdk_includes, 
            compiler_path: local_msvc_install, 
            msvc_includes_path: local_msvc_includes,
            msvc_version, 
            env_args: "".to_string()
        }
    }
}

fn get_winsdk_includes_path() -> Option<Vec<String>> {
    use std::os::windows::ffi::OsStrExt;
    use std::iter::once;
    use std::ptr::null_mut;
    unsafe {

        //HKEY_LOCAL_MACHINE\SOFTWARE\Wow6432Node\Microsoft\Microsoft SDKs\Windows\v10
        let sub_key: Vec<u16> = std::ffi::OsStr::new(r"SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0").encode_wide().chain(once(0)).collect();
        let mut phk_result: winapi::shared::minwindef::HKEY = null_mut();
        let open_status = winapi::um::winreg::RegOpenKeyW(winapi::um::winreg::HKEY_LOCAL_MACHINE, 
            sub_key.as_ptr(), &mut phk_result);
        
        if open_status == winapi::shared::winerror::SEC_E_OK {

            let value_name: Vec<u16> = std::ffi::OsStr::new("InstallationFolder").encode_wide().chain(once(0)).collect();

            let mut dword: winapi::shared::minwindef::DWORD = 128;
            let mut data = vec![0; 128 as usize];
            let mut reg_sz =  winapi::um::winnt::REG_SZ;
            let query_status = winapi::um::winreg::RegQueryValueExW(phk_result, value_name.as_ptr(), null_mut(), 
              &mut reg_sz, data.as_mut_ptr(), &mut dword);

            let mut winkits_path = String::new();
            if query_status == winapi::shared::winerror::SEC_E_OK {
                data.set_len(dword as usize);
                let words = std::slice::from_raw_parts(data.as_ptr() as *const u16, data.len() / 2);
                winkits_path = String::from_utf16_lossy(words);

                winkits_path = winkits_path.trim_end_matches('\0').to_string();
            }
            else {
                println!("get regedit InstallationFolder failed . error code: {:?}", query_status);
            }
            let product_version: Vec<u16> = std::ffi::OsStr::new("ProductVersion").encode_wide().chain(once(0)).collect();

            let mut version_data = vec![0; 32];
            let mut version_len : winapi::shared::minwindef::DWORD = 32;
            let query_version_status = winapi::um::winreg::RegQueryValueExW(phk_result, product_version.as_ptr(), null_mut(),
             &mut reg_sz, version_data.as_mut_ptr(),  &mut version_len);

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
                let include_path = std::path::Path::new(winkits_path.as_str()).join("Include").join(winkits_version.as_str());
               
                let mut includes_path : Vec<String> = Vec::new();
                let cppwinrt_include = include_path.join("cppwinrt");

                if cppwinrt_include.exists() {
                    includes_path.push(cppwinrt_include.into_os_string().into_string().unwrap());
                }

                let shared_include = include_path.join("shared");
                if shared_include.exists() {
                    includes_path.push(shared_include.into_os_string().into_string().unwrap());
                }

                let ucrt_include = include_path.join("ucrt");
                if ucrt_include.exists() {
                    includes_path.push(ucrt_include.into_os_string().into_string().unwrap());
                }

                let um_include = include_path.join("um");
                if um_include.exists() {
                    includes_path.push(um_include.clone().into_os_string().into_string().unwrap());
                }

                if um_include.join("winsdkver.h").exists() || um_include.join("windows.h").exists() {
                    //println!("winsdkver.h and windows.h files exists");
                }

                let winrt_include = include_path.join("winrt");
                if winrt_include.exists() {
                    includes_path.push(winrt_include.into_os_string().into_string().unwrap());
                }
                return Some(includes_path)
            }
             
        } else {
            println!("open regedit failed, error: {:?}.", open_status);
        }
        return None
    }
}

fn get_msvc_version() -> Option<String> {
    use std::io::Read;

    match get_visualstudio_install_path() {
        None => {
            println!("get vs install path failed");
            return None; 
        },
        Some(vs_install_path) => {
            //C:\Program Files (x86)\Microsoft Visual Studio\2019\Enterprise\VC\Auxiliary\Build
            let vc_auxiliary_build_path = vs_install_path.join("VC").join("Auxiliary").join("Build")
            .join("Microsoft.VCToolsVersion.default.txt");
            if vc_auxiliary_build_path.exists() {
                let vc_version_file = std::fs::File::open(vc_auxiliary_build_path);
                match vc_version_file {
                    Ok(mut file) => {

                        let mut vc_tools_version = String::new();
                        let _version = file.read_to_string(&mut vc_tools_version);
                        vc_tools_version = vc_tools_version.replace("\r\n", "");
                        if !vc_tools_version.is_empty() {
                            return Some(vc_tools_version)
                        }
                        else {
                            return None;
                        };
                    },
                    Err(error) => {
                        println!("do not open vc auxiliary build path: {:?}", error);
                        return None;
                    },
                };
            } else {
                println!("vc tools version default do not exist: {:?}", vc_auxiliary_build_path);
                return None;
            }
        },
    };
}

fn get_local_vc_tools() -> Option<std::path::PathBuf> {

    match get_visualstudio_install_path() {
        Some(vs_install_path) => {
            //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools
            let tools_path = vs_install_path.join("VC").join("Tools");
            return Some(tools_path);
        },
        None => {
            return None;
        },
    }
}

fn get_local_msvc_path() -> Option<std::path::PathBuf> {

    match get_local_vc_tools() {
        Some(path) => {
            match get_msvc_version() {
                Some(version) => {
                    let msvc_path = path.join("MSVC").join(version);
                    if msvc_path.is_dir() {
                        return Some(msvc_path);
                    }
                    else {
                        return None;
                    }
                },
                None => {
                    println!("get_msvc_version return none");
                    return None;
                },
            }
        },
        None => {
            return None;
        },
    }
}

fn get_local_msvc_include_files_path() -> Option<std::path::PathBuf> {

    match get_local_msvc_path() {
        Some(msvc_path) => {
            let msvc_include_files_path = msvc_path.join("include");
            if msvc_include_files_path.is_dir() {
                return Some(msvc_include_files_path);
            }
            else {
                return None;
            }
        },
        None => {
            println!("get_local_msvc_path return none");
            return None;
        },
    }
}

fn get_visualstudio_install_path() -> Option<std::path::PathBuf> {

    //"C:\Program Files (x86)"
    let program_files_path = std::env::var_os("PROGRAMFILES(X86)").unwrap();
    let vswhere = std::path::Path::new(program_files_path.to_str().unwrap()).join("Microsoft Visual Studio")
    .join("Installer").join("vswhere.exe");

    let exist = vswhere.exists();
    if exist {
            
        let result =  std::process::Command::new(vswhere)
        .arg("-latest")
        .arg("-products").arg("*")
        .arg("-requires").arg("Microsoft.VisualStudio.Component.VC.Tools.x86.x64")
        .arg("-property").arg("installationPath")
        .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    let vs_install_path = String::from_utf8_lossy(&output.stdout);
                    let pure_vs_install_path = std::path::PathBuf::from( vs_install_path.replace("\r\n", ""));
                    if pure_vs_install_path.is_dir() {
                        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\
                        return Some(pure_vs_install_path);
                    }
                    else {
                        return None;
                    }
                }
                else {
                    println!("execute vswhere.exe result have error: {:?}", output.status.code());
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


fn get_local_msvc_bin_path() -> Option<std::path::PathBuf> {

    match get_local_msvc_path() {
        Some(msvc_path) => {
            //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin
            let bin_path = msvc_path.join("bin");
            if bin_path.is_dir() {
                return Some(bin_path);
            }
            else {
                return None;
            }
        },
        None => {
            return None;
        },
    }
}