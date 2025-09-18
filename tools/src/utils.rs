
pub fn access_working_path(app: &str) -> std::option::Option<std::string::String> {
    let mut dir = std::env::current_exe().unwrap();

    let path = dir.join(app);
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
pub fn access_or_create_working_path(app: &str) -> String {
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
        let dir = access_or_create_working_path("Replica");
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

pub fn normalize_lexical<P: AsRef<std::path::Path>>(p: P) -> std::path::PathBuf {
    let mut prefix: Option<std::path::PrefixComponent> = None;
    let mut saw_root = false;
    let mut parts: Vec<std::ffi::OsString> = Vec::new();

    for c in p.as_ref().components() {
        match c {
            std::path::Component::Prefix(pf) => { prefix = Some(pf); }
            std::path::Component::RootDir => { saw_root = true; parts.clear(); }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !parts.is_empty() {
                    parts.pop();
                } else if !saw_root {
                    parts.push("..".into());
                }
            }
            std::path::Component::Normal(s) => parts.push(s.to_os_string()),
        }
    }

    let mut out = std::path::PathBuf::new();
    if let Some(pf) = prefix { out.push(pf.as_os_str()); }
    if saw_root { out.push(std::path::MAIN_SEPARATOR.to_string()); }
    for s in parts { out.push(s); }
    out
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_normalize_lexical() {

        let path = r"C:\Users\user\..";
        let normalized = normalize_lexical(path);
        assert_eq!(normalized.to_str().unwrap(), r"C:\Users\");

        let path = r"C:\Users\user\..\";
        let normalized = normalize_lexical(path);
        assert_eq!(normalized.to_str().unwrap(), r"C:\Users\");

        let path = r"C:\Users\user\..\Documents\..\Desktop\file.txt";
        let normalized = normalize_lexical(path);
        assert_eq!(normalized.to_str().unwrap(), r"C:\Users\Desktop\file.txt");
    }
}