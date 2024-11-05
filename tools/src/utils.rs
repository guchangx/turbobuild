//TODO use access replace get
pub fn get_working_path(app: std::string::String) -> std::option::Option<std::string::String> {

    let mut dir = std::env::current_exe().unwrap();

    let path = dir.join(app.clone());
    if path.exists()
    {
        return Some(path.display().to_string());
    }
    else
    {
        dir.pop();
        if dir.ends_with("build") {
            dir.pop();
        }
    
        let path = dir.join(app);
        println!("path: {:?}", path);
        if path.exists() {
            return Some(path.display().to_string());
        }
        else {
            return None;
        }
    }
}

pub fn get_or_create_working_path(app: &str) -> String {
    let mut dir = std::env::current_exe().unwrap();
    log::info!("current dir {:?}", dir);
    
    let mut path = "";
    if dir.components().any(|item| item.as_os_str() == "deps") {
        dir.pop();
        dir.pop();
        dir.push(app);
        if !dir.exists() {
            std::fs::create_dir(&dir).unwrap();
        }
        path = dir.to_str().unwrap();
    }
    else {
        dir.pop();
        dir.push(app);
        
        if !dir.exists() {
            std::fs::create_dir(&dir).unwrap();
        }
        path = dir.to_str().unwrap();
    }

    return path.to_string();
}