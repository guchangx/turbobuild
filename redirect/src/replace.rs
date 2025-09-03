
const PROJECT: &str = "Project";
pub struct Model {} 

impl Model {
    
    pub fn fetch_local_replica_project_path(origin_file_path: &str) -> Option<String> {
        
        if let Some(replica_dir) = crate::REPLICADIR.get() {
            if let Some(solution_name) = crate::SOLUTIONNAME.get() {
                if let Some(point) = origin_file_path.find(solution_name) {
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

    pub fn fetch_local_replica_sources_dir() -> Option<String> {
        if crate::WORKINGDIR.is_empty() || crate::GENERATEDDIR.get().is_none() {
            return None;
        }
        else {
            let dir = Self::fetch_local_replica_project_path(crate::GENERATEDDIR.get().unwrap());
            return dir;
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
    else if path.ends_with(".h") || path.ends_with(".hpp") {
        return false;
    }
    else if path.ends_with(".cpp") || path.ends_with(".cxx") || path.ends_with(".c") || path.ends_with(".cc") {
        if crate::SOLUTIONNAME.get().is_some() {
            if let Some(name) = std::path::Path::new(path).file_name() {
                let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&name);
                *path = modified.to_string_lossy().to_string();
            };
            return true;
        }
        else  {
            return false;
        }
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
            crate::log!(warn, "project name and replica dir in model are not ready");
            return false;
        }
    }
    else if path.ends_with("warning_suppression.txt") {
        return false;
    }
    else {
        return false;
    }
}

#[derive(PartialEq, Debug)]
pub enum ReplaceDirResult {
    Success,
    NoMatch,
    IncludesDir,
    FilePath,
}

pub fn replace_dir(path: &mut String) -> ReplaceDirResult {

    if std::path::Path::new(path).extension().is_some() {
        return ReplaceDirResult::FilePath;
    }
    else if path.contains(r"AppData\Local\Temp\") {
        return ReplaceDirResult::NoMatch;
    }
    else if path.contains(r"Replica\MSVC") || path.contains(r"Replica\Windows Kits") {
        return ReplaceDirResult::NoMatch;
    }
    else if path.starts_with(r"\??\pipe\") {
        return ReplaceDirResult::NoMatch;
    }
    else if crate::SOLUTIONNAME.get().is_some() && path.contains(crate::SOLUTIONNAME.get().unwrap()) {

        if path.ends_with(r"\") {
            crate::log!(warn, "replace with  {:?}, path: {}", crate::INCLUDES.get(), path);

            let target = &path[4..path.len() - 1];
            
            if crate::INCLUDES.get().unwrap().iter().any(|item| item == target || target.starts_with(item)) {
                return ReplaceDirResult::IncludesDir;
            }
            else {
                let generated = std::path::Path::new(crate::GENERATEDDIR.get().unwrap());
                if generated.is_absolute() {
                    let modified = Model::fetch_local_replica_sources_dir();
                    if let Some(modified) = modified {
                        if path.starts_with(r"\??\") {
                            *path = format!(r"\??\{}", modified);
                        }
                        else {
                            *path = modified;
                        }
                        return ReplaceDirResult::Success;
                    } else {
                        crate::log!(warn, "replace sources dir is not ready, original: {}", path);
                        return ReplaceDirResult::NoMatch;
                    }
                } 
                else {
                    let modified =std::path::Path::new(&*crate::WORKINGDIR).join(generated);
                    let modified = modified.to_string_lossy().to_string();
                    if path.starts_with(r"\??\") {
                        *path = format!(r"\??\{}", modified);
                    }
                    else {
                        *path = modified;
                    }

                    return ReplaceDirResult::Success;
                }
            }
        }
        else {

            let len = path.rfind(r"\").unwrap();
            let target = &path[4..len + 1];
            let has = crate::INCLUDES.get().unwrap().iter().any(|item| item == target || target.starts_with(item));
            crate::log!(warn, "replacereplace: {:?}, path: {} target: {} has: {}", crate::INCLUDES.get(), path, target, has);

            if has {
                //.h file in include dir
                return ReplaceDirResult::NoMatch;
            }
            else {
                let generated = std::path::Path::new(crate::GENERATEDDIR.get().unwrap());
                if generated.is_absolute() {
                    let modified = Model::fetch_local_replica_sources_dir();
                    if let Some(modified) = modified {
                        if path.starts_with(r"\??\") {
                            *path = format!(r"\??\{}", modified);
                        }
                        else {
                            *path = modified;
                        }
                        return ReplaceDirResult::Success;
                    } else {
                        crate::log!(warn, "replace sources dir is not ready, original: {}", path);
                        return ReplaceDirResult::NoMatch;
                    }
                } 
                else {
                    let modified =std::path::Path::new(&*crate::WORKINGDIR).join(generated);
                    let modified = modified.to_string_lossy().to_string();
                    if path.starts_with(r"\??\") {
                        *path = format!(r"\??\{}", modified);
                    }
                    else {
                        *path = modified;
                    }

                    return ReplaceDirResult::Success;
                }
            }
        }
    }
    else {
        return ReplaceDirResult::NoMatch;
    }
}



#[cfg(test)]
mod tests {

    #[test]
    fn load_replica() {
        let path = r"\??\E:\TestFuture\ZLMediaKit\3rdpart\media-server\libmov\include\mov-udta.h";
        println!("original: {}", path);
        let index = path.rfind(r"\").unwrap();
        println!("modified: {}", path[4..index + 1].to_string());
    }
}