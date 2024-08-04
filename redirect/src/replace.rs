
pub fn replace(path: std::string::String) -> std::string::String {

    if path.ends_with(".cpp") || path.ends_with(".cxx") || path.ends_with(".c") || path.ends_with(".cc") || path.ends_with(".h")
        || path.ends_with(".hpp") || path.ends_with(".hh")
    {
        return path.to_string();
    }
    else {
        return "".to_string();
    }
}
