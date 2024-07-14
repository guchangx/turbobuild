
pub fn get_working_path(app: std::string::String) -> std::string::String {
    let dir = std::env::current_dir().unwrap();
    let dir = dir.join(app);
    return dir.display().to_string();
}