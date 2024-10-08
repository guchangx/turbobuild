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
        dir.pop();
    
        let path = dir.join(app);
        if path.exists() {
            return Some(path.display().to_string());
        }
        else {
            return None;
        }
    }
}