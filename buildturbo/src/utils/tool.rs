
pub fn get_working_path(app: std::string::String) -> std::string::String {
    let mut dir = std::env::current_dir().unwrap();
    dir.push("target\\debug\\");
    let path = dir.join(app); 
    if !path.exists() {
        println!("{} not found", path.display());
    }

    return path.display().to_string();
}