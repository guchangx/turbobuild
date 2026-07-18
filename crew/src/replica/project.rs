
pub struct Property {
    solution_name: String,
    real_project_path: String,
    replica_project_dir: String,
} 

impl Property {
    pub fn new(solution_name: &str, real_project_path: &str) -> Self {
        let replica_project_dir = Self::fetch_local_replica_dir();
        Property {
            solution_name: solution_name.to_string(),
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
            return std::path::PathBuf::from(format!("{}/{}", self.replica_project_dir, self.solution_name));
        }
        else {
            if let Some(point) = self.real_project_path.find(&self.solution_name) {
                let tail = self.real_project_path[point..].to_string();
                return std::path::PathBuf::from(self.replica_project_dir).join("Project").join(tail);
            }
            else
            {
                let index = self.real_project_path.find(":\\");
                if let Some(i) = index {
                    let reset = &self.real_project_path[i + 2..];
                    return std::path::PathBuf::from(self.replica_project_dir).join(reset);
                }
                else {
                    return std::path::PathBuf::from(self.replica_project_dir).join(self.real_project_path);
                }
            }
        }
    }
}

#[test]
fn redirect_path_or_dir_test() {
    let solution = "MediaKit";
    let path = "D:\\Software\\ffmpeg-master-latest-win64-gpl-shared\\include\\libswscale\\swscale.h";
    let p: Property = Property::new(solution, path);
    let repath = p.fetch_local_replica_project_path();
    println!("redirect_path_or_dir_test: {:?}", repath);
}