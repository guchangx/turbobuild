
struct ReplaceFile {
    map: std::collections::HashMap<std::string::String, std::string::String>,
}

pub fn replace(path: std::string::String) -> std::string::String {

    if path.ends_with(".cpp") || path.ends_with(".cxx") || path.ends_with(".c") || path.ends_with(".cc") {
        if path.ends_with("test.cpp") {
            return "fake_test.cpp".to_string();
        }
        else {
            return path;
        }
    }
    else {
        return "".to_string();
    }
}
