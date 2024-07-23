
pub fn get_working_path(app: std::string::String) -> std::string::String {
    let mut dir = std::env::current_dir().unwrap();
    dir.push("target\\debug\\");
    let path = dir.join(app);

    //"E:\\WorkSpace\\NewTest\\Discord-ModLoader\\target\\debug\\libmodhook.dll"
    //"E:\\TestFuture\\turbobuild\\target\\debug\\redirect64.dll"
    let path = std::path::Path::new("E:\\TestFuture\\turbobuild\\target\\debug\\redirect64.dll");
    if !path.exists() {
        println!("{} not found", path.display());
    }
    return path.display().to_string();
}