
static MODEL: std::sync::LazyLock<std::sync::Mutex<Model>> = std::sync::LazyLock::new(|| {
    std::sync::Mutex::new(Model {
        project_name: "".to_string(),
        replica_project_dir: "".to_string(),
    })
});

pub struct Model {
    project_name: String,
    replica_project_dir: String,
} 

impl Model {
    pub fn new(project_name: String) -> Self {
        //E:\TestFuture\GammaRay\GammaRayTool\3rdparty\kde\kmodelindexproxymapper.cpp
        let replica_project_dir = Self::fetch_local_replica_project_dir();
        let mut dir = std::path::PathBuf::from(replica_project_dir);
        dir.push(&project_name);
            
        if dir.exists() {
            if let Ok(_) = std::fs::create_dir_all(&dir) {
                return Model {
                    project_name,
                    replica_project_dir: dir.to_string_lossy().to_string(),
                }
            }
        }
        
        return Model {
            project_name,
            replica_project_dir: "".to_string(),
        }
    }

    fn fetch_local_replica_project_dir() -> String {
        if let Some(replica) = crate::REPLICADIR.lock().unwrap().clone() {
            let dir = replica + "Projet";
            return dir;
        }
        else {
            return "".to_string();
        }
    }
    
    pub fn fetch_local_replica_project_path(&self, origin_file_path: &str) -> std::path::PathBuf {
        if self.replica_project_dir.is_empty() {
            return std::path::PathBuf::from("");
        }
        else {
            if let Some(point) = origin_file_path.find(&self.project_name) {
                let tail = origin_file_path[point..].to_string();
                println!("real project path: {:?}", origin_file_path);

                if origin_file_path.contains("\\??\\")
                {
                    let nt_dir = format!("\\??\\{}", self.replica_project_dir);
                    println!("nt dir {}", nt_dir);

                    return std::path::PathBuf::from(nt_dir).join("Project").join(tail);
                }
                else 
                {
                    return std::path::PathBuf::from(&self.replica_project_dir).join("Project").join(tail);
                }
            }
            else
            {
                return std::path::PathBuf::from(&self.replica_project_dir).join("Project");
            }
        }
    }

}

pub fn replace(path: &mut String) -> bool {

    if path.ends_with("clui.dll") {
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.39.33519\bin\Hostx64\x64\1033\clui.dll
        if let Some(index) = path.find("MSVC") {
            let clui = &path[(index + "MSVC".len())..];
            let modified = format!(r#"{}{}{}"#, crate::REPLICADIR.lock().unwrap().clone().unwrap(), "MSVC", clui);
            *path = modified;
        }
        return true;
    }
    if path.ends_with(".dll") || path.ends_with("_PIPE") {
        return false;
    }
    else if path.ends_with(".cpp") || path.ends_with(".cxx") || path.ends_with(".c") || path.ends_with(".cc") {
        return false;
    }
    else if path.ends_with(".obj") {
        return false;
    } 
    else if path.ends_with(".pdb") {
        return false;
    }
    else if path.ends_with(".i") {
        match &*crate::PROJECTNAME.lock().unwrap() {
            Some(project_name) => {
                if MODEL.lock().unwrap().project_name == project_name.to_string() {
                    let modified = MODEL.lock().unwrap().fetch_local_replica_project_path(&path);
                    *path = modified.to_string_lossy().to_string();
                }
                else {
                    let project = Model::new(project_name.clone());    
                    *MODEL.lock().unwrap() = project;
                    let modified = MODEL.lock().unwrap().fetch_local_replica_project_path(&path);
                    *path = modified.to_string_lossy().to_string();
                }
            },
            None => {},
        }
        
        return true;
    }
    else {
        return false;
    }
}

pub fn replace_dir(path: std::string::String) -> std::string::String {

    if path.ends_with(".dll")
    {
        return path;
    }
    else {
        return path;
    }
}