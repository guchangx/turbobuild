//TODO use access replace get
pub fn get_working_path(app: std::string::String) -> std::option::Option<std::string::String> {
    //TODO should be use &str
    let mut dir = std::env::current_exe().unwrap();

    let path = dir.join(app.clone());
    if path.exists()
    {
        return Some(path.display().to_string());
    }
    else
    {
        dir.pop();
        if dir.ends_with("build") || dir.ends_with("deps"){
            dir.pop();
        }
    
        let path = dir.join(app);
        log::debug!("path: {:?}", path);
        if path.exists() {
            return Some(path.display().to_string());
        }
        else {
            return None;
        }
    }
}

#[allow(unused_assignments)]
pub fn get_or_create_working_path(app: &str) -> String {
    let mut dir = std::env::current_exe().unwrap();
    log::info!("current dir {:?}", dir);
    
    let mut path = "";
    if dir.components().any(|item| item.as_os_str() == "deps") {
        dir.pop();
        dir.pop();
        dir.push(app);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        path = dir.to_str().unwrap();
    }
    else {
        dir.pop();
        dir.push(app);
        
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        path = dir.to_str().unwrap();
    }

    return path.to_string();
}

static REPLICADIR: std::sync::Mutex<std::option::Option<String>> = std::sync::Mutex::new(None);
pub fn access_replica_dir() -> String {
    if REPLICADIR.lock().unwrap().is_some() {
        return REPLICADIR.lock().unwrap().clone().unwrap();
    }
    else {
        let dir = get_or_create_working_path("Replica");
        REPLICADIR.lock().unwrap().replace(dir.clone());
        return dir;
    }
}


pub fn get_winapi_error_message(error: u32) -> String {
    use std::os::windows::ffi::OsStringExt;
    unsafe {
        let mut buffer = vec![0u16; 256];
        let size =  winapi::um::winbase::FormatMessageW(
            winapi::um::winbase::FORMAT_MESSAGE_FROM_SYSTEM,
            std::ptr::null_mut(),
            error,
            0,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            std::ptr::null_mut(),
        );
        if size != 0 {
            std::ffi::OsString::from_wide(&buffer[..size as usize])
            .to_string_lossy()
            .into_owned()
        }
        else {
            "".to_string()
        }
    }
}
