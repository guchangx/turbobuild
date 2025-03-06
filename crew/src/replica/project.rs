
pub struct Property {
    project_name: String,
    real_project_path: String,
    replica_project_dir: String,
} 

impl Property {
    pub fn new(project_name: &str, real_project_path: &str) -> Self {
        let replica_project_dir = Self::fetch_local_replica_dir();
        Property {
            project_name: project_name.to_string(),
            real_project_path: real_project_path.to_string(),
            replica_project_dir,
        }
    }

    fn fetch_local_replica_dir() -> String {

        match tools::utils::access_working_path("Replica") {
            Some(path) => {
                return path;
            },
            None => {
                return "".to_string();
            }
        }
    }

    pub fn fetch_local_replica_project_path(self) -> std::path::PathBuf {
        if self.replica_project_dir.is_empty() {
            return std::path::PathBuf::from(format!("{}/{}", self.replica_project_dir, self.project_name));
        }
        else {
            if let Some(point) = self.real_project_path.find(&self.project_name) {
                let tail = self.real_project_path[point..].to_string();
                return std::path::PathBuf::from(self.replica_project_dir).join("Project").join(tail);
            }
            else
            {
                log::warn!("can not find project name {:?} in real project path {}.", self.project_name, self.real_project_path);
                return std::path::PathBuf::from(self.replica_project_dir).join("Project");
            }
        }
    }

}