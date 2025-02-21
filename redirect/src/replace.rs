
use crate::log;

const PROJECT: &str = "Project";

pub struct Model {} 

impl Model {
    
    pub fn fetch_local_replica_project_path(origin_file_path: &str) -> Option<String> {
        
        if let Some(replica_dir) = crate::REPLICADIR.get() {
            if let Some(project_name) = crate::PROJECTNAME.get() {
                if let Some(point) = origin_file_path.find(project_name) {
                    let tail = origin_file_path[point..].to_string();
    
                    if origin_file_path.contains("\\??\\")
                    {
                        let path = format!(r"\??\{}\{}\{}", replica_dir, PROJECT, tail);
                        return Some(path);
                    }
                    else
                    {
                        let path = format!(r"{}\{}\{}", replica_dir, PROJECT, tail);
                        return Some(path);
                    }
                }
                else
                {
                    let path = origin_file_path.to_owned();
                    return Some(path);
                }
            }
            return None;
        }
        else {
            return None;
        }
    }

}

pub fn replace(path: &mut String) -> bool {

    if path.ends_with("clui.dll") {
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.39.33519\bin\Hostx64\x64\1033\clui.dll
        if let Some(index) = path.find("MSVC") {
            if let Some(replica_dir) = crate::REPLICADIR.get() {
                let clui = &path[(index + "MSVC".len())..];

                let modified = format!(r#"{}\{}{}"#, replica_dir, "MSVC", clui);
                *path = modified;
                return true;
            }
            else {
                return false;
            }
        }
        return false;
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
        let modified = Model::fetch_local_replica_project_path(&path);
        if let Some(modified) = modified {
            *path = modified;
            return true;
        } else {
            log!(warn, "project name and replica dir in model are not ready");
            return false;
        }
    }
    else {
        return false;
    }
}

pub fn replace_dir(path: &mut String) -> bool {

    if path.ends_with("clui.dll") {
        return false;
    }
    else if path.contains(r"AppData\Local\Temp\") {
        return false;
    }
    else if path.ends_with(".i") {
        return false;
    }
    else if path.starts_with("\\??\\pipe\\") {
        return false;
    }
    else if crate::PROJECTNAME.get().is_some() && path.contains(crate::PROJECTNAME.get().unwrap()) {
        let modified = Model::fetch_local_replica_project_path(&path);
        if let Some(modified) = modified {
            *path = modified;
            return true;   
        } else {
            log!(warn, "project name and replica dir in model are not ready");
            return false;
        }
    } 
    else {
        return false;
    }
}