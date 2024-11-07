pub struct Model {
    project_name: String,
    real_project_path: String,
    replica_project_dir: String,
} 

impl Model {
    pub fn new(_project_name: String, real_project_path: String, _replica_project_dir: String) -> Self {
        let replica_project_dir = Self::fetch_local_replica_dir();
        let project_name = "GammaRayTool".to_string();
        Model {
            project_name,
            real_project_path,
            replica_project_dir,
        }
    }

    fn fetch_local_replica_dir() -> String {
        return r"C:\WorkSpace\MyWork\turbobuild\target\Replica".to_string();
    }

    pub fn fetch_local_replica_project_path(self) -> std::path::PathBuf {
        if self.replica_project_dir == "" {
            return std::path::PathBuf::from(self.real_project_path);
        }
        else {
            if let Some(point) = self.real_project_path.find(&self.project_name) {
                let tail = self.real_project_path[point..].to_string();
                println!("real project path: {:?}", self.real_project_path);

                if self.real_project_path.contains("\\??\\")
                {
                    let nt_dir = format!("\\??\\{}", self.replica_project_dir);
                    println!("nt dir {}", nt_dir);

                    return std::path::PathBuf::from(nt_dir).join("project").join(tail);
                }
                else 
                {
                    return std::path::PathBuf::from(self.replica_project_dir).join("project").join(tail);
                }
            }
            else
            {
                return std::path::PathBuf::from(self.replica_project_dir).join("project");
            }
        }
    }

}

struct ReplaceFile {
    map: std::collections::HashMap<std::string::String, std::string::String>,
}

pub fn replace(path: std::string::String) -> std::string::String {

    if path.ends_with(".cpp") || path.ends_with(".cxx") || path.ends_with(".c") || path.ends_with(".cc") 
    || path.ends_with(".i") || path.ends_with(".obj") || path.ends_with(".pdb") {
        if path.ends_with("test.cpp") {
            return "E:\\TestFuture\\buildturbo\\draft\\fake_test.cpp".to_string();
        }
        else if path.ends_with(".i") {
            let project = Model::new("".to_string(), path, "".to_string());
            let path = project.fetch_local_replica_project_path();
            return path.display().to_string();
        }
        else if path.contains("GammaRayTool")
        {
            return path;
        }
        else {
            return path;
        }
    }
    else {
        return "".to_string();
    }
}


pub fn replace_dir(path: std::string::String) -> std::string::String {

    if path.contains("fake_draft") {
        return path.replace("fake_draft", "draft");
    }
    else if path.contains("GammaRayTool")
    {
        let project = Model::new("".to_string(), path, "".to_string());
        let path = project.fetch_local_replica_project_path();
        return path.display().to_string();
    }
    else {
        return "".to_string();
    }
}