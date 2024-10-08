
pub struct Property {
    project_name: String,
    real_project_path: String,
    replica_project_dir: String,
} 

impl Property {
    pub fn new(_project_name: String, real_project_path: String, _replica_project_dir: String) -> Self {
        let replica_project_dir = Self::fetch_local_replica_dir();
        let project_name = "GammaRayTool".to_string();
        Property {
            project_name,
            real_project_path,
            replica_project_dir,
        }
    }

    fn fetch_local_replica_dir() -> String {

        match common::util::get_working_path("Replica".to_string()) {
            Some(path) => {
                return path;
            },
            None => {
                return "".to_string();
            }
        }
    }

    pub fn fetch_local_replica_project_path(self) -> std::path::PathBuf {
        if self.replica_project_dir == "" {
            return std::path::PathBuf::from(self.real_project_path);
        }
        else {
            if let Some(point) = self.real_project_path.find(&self.project_name) {
                let tail = self.real_project_path[point..].to_string();
                return std::path::PathBuf::from(self.replica_project_dir).join("project").join(tail);
            }
            else
            {
                return std::path::PathBuf::from(self.replica_project_dir).join("project");
            }
        }
    }

}