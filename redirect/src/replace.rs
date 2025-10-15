
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

#[derive(PartialEq, Debug)]
pub enum ReplaceResult {
    Success,
    NoMatch,
    FilePath,
}

pub fn replace(path: &mut String) -> ReplaceResult {

    if path.contains(r"AppData\Local\Temp\") || path.contains(r"Replica\MSVC") || path.contains(r"Replica\Windows Kits") {
        return ReplaceResult::NoMatch;
    }
    else if path.ends_with("clui.dll") {
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.39.33519\bin\Hostx64\x64\1033\clui.dll
        if let Some(index) = path.find("MSVC") {
            if let Some(replica_dir) = crate::REPLICADIR.get() {
                let clui = &path[(index + "MSVC".len())..];

                let modified = format!(r#"{}\{}{}"#, replica_dir, "MSVC", clui);
                *path = modified;
                return ReplaceResult::Success;
            }
            else {
                return ReplaceResult::NoMatch;
            }
        }
        return ReplaceResult::NoMatch;
    }
    else if let Some(extension) = std::path::Path::new(path).extension() {
        if extension == "h" || extension == "hpp"  || extension == "inl" {
            return ReplaceResult::NoMatch;
        }
        else if extension == "pdb" {
            if let Some(replica_pdbpath) = crate::REPLICA_PDBPATH.get() {
                *path = replica_pdbpath.to_string();
                return ReplaceResult::Success;
            }
            else if let Some(solution) = crate::SOLUTIONNAME.get() {
                if let Some(index) = path.find(solution) {
                    let tail = &path[index..];
                    let modified = format!(r"{}\{}\{}", crate::REPLICADIR.get().unwrap(), "Project", tail);
                    *path = modified;
                    return ReplaceResult::Success;
                }
                else {
                    return ReplaceResult::NoMatch;
                }
            }
            else {
                return ReplaceResult::NoMatch;
            }
        }
        else if extension == "c" || extension == "cpp" || extension == "cxx" || extension == "cc" {
            let has = crate::SOURCES.get().unwrap().iter().find(|&item| {
                path.starts_with(item.to_str().unwrap())
            });

            if has.is_some() {
                if crate::SOLUTIONNAME.get().is_some() && path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                    Model::fetch_local_replica_project_path(&path).map_or(ReplaceResult::NoMatch, |modified| {
                        *path = modified;
                        return ReplaceResult::FilePath;
                    })
                }
                else if crate::REPLICADIR.get().is_some() {
                    if let Some(name) = std::path::Path::new(path).file_name() {
                        let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&name);
                        *path = modified.to_string_lossy().to_string();
                    };
                    return ReplaceResult::FilePath;
                }
                else {
                    return ReplaceResult::NoMatch;
                }   
            }
            else {
                return ReplaceResult::NoMatch;
            }
        }
        else if extension == "obj" {
            return ReplaceResult::NoMatch;
        }
        else if extension == "i" {
            let modified = Model::fetch_local_replica_project_path(&path);
            if let Some(modified) = modified {
                *path = modified;
                return ReplaceResult::Success;
            } else {
                crate::log!(warn, "project name and replica dir in model are not ready");
                return ReplaceResult::NoMatch;
            }
        }
        else if path.ends_with("warning_suppression.txt") {
            return ReplaceResult::NoMatch;
        }
        else {
            return ReplaceResult::NoMatch;
        }
    }
    else {
        return ReplaceResult::NoMatch;
    }
}

#[derive(PartialEq, Debug)]
pub enum ReplaceDirResult {
    Success,
    NoMatch,
    IncludesDir,
    FilePath,
    NeedObtain(String),
}

pub fn replace_dir(path: &mut String) -> ReplaceDirResult {

    if path.contains(r"AppData\Local\Temp\") {
        return ReplaceDirResult::NoMatch;
    }
    else if path.starts_with(r"\??\pipe\") {
        return ReplaceDirResult::NoMatch;
    }
    else if let Some(extension) = std::path::Path::new(path).extension() {
        if extension == "h" || extension == "hpp" || extension == "inl" {
            if path.contains(r"Replica\MSVC") || path.contains(r"Replica\Windows Kits") {
                return ReplaceDirResult::FilePath;
            }
            //TODO: files has been redirect, need not replace again.
            else if crate::REPLICADIR.get().is_some() {
                let stempath = std::path::Path::new(&path[4..]);
                let stempath = stempath.with_extension("");
                let sources = crate::SOURCES.get().unwrap();
                if sources.contains(stempath.as_os_str()) {
                    if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                        Model::fetch_local_replica_project_path(&path).map_or(ReplaceDirResult::Success, |modified| {
                            *path = modified;
                            return ReplaceDirResult::Success;
                        })
                    }
                    else {
                        let name = std::path::Path::new(path).file_name().unwrap();
                        let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&name);
                        let modified = modified.to_string_lossy().to_string();
                        
                        *path = format!(r"\??\{}", modified);
                        return ReplaceDirResult::Success;
                    }
                }
                else {
                    //TODO: i do not know how to handle this case now, .h file should in generated dir or in self project dir.
                    //TODO: if a .h file in sources dir, but name is source file name, should not obtain again.
                    if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                        Model::fetch_local_replica_project_path(&path).map_or(ReplaceDirResult::Success, |modified| {
                            return ReplaceDirResult::NeedObtain(modified);
                        })
                    }
                    else {
                        if let Some(index) = path.rfind(|c| c == '\\') {
                            if let Some(index_) = path[..index].rfind(|c| c == '\\') {
                                let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&path[index_..]).to_string_lossy().to_string();
                                let modified = format!(r"\??\{}", modified);
                                return ReplaceDirResult::NeedObtain(modified);
                            }
                            else {
                                let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&path[index..]).to_string_lossy().to_string();
                                let modified = format!(r"\??\{}", modified);
                                return ReplaceDirResult::NeedObtain(modified);
                            }
                        }
                        else {
                            return ReplaceDirResult::FilePath;
                        }
                    }   
                }
            }
            else if extension == "cpp" {
                if path.contains("mocs_") || path.contains("qrc_") {
                    if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                        Model::fetch_local_replica_project_path(&path).map_or(ReplaceDirResult::Success, |modified| {
                            return ReplaceDirResult::NeedObtain(modified);
                        })
                    }
                    else {
                        if let Some(name) = std::path::Path::new(path).file_name() {
                            let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&name).to_string_lossy().to_string();
                            let modified = format!(r"\??\{}", modified);
                            return ReplaceDirResult::NeedObtain(modified);
                        }
                        else {
                            return ReplaceDirResult::FilePath;
                        }
                    }
                }
                else {
                    return ReplaceDirResult::FilePath;
                }
            }
            else {
                return ReplaceDirResult::FilePath;
            }
        }
        else {
            return ReplaceDirResult::FilePath;
        }
    }
    else {
        if path.contains(r"Replica\MSVC") || path.contains(r"Replica\Windows Kits") {
            return ReplaceDirResult::NoMatch;
        }
        else if crate::SOLUTIONNAME.get().is_some() && path.contains(crate::SOLUTIONNAME.get().unwrap()) {

            let target = std::path::PathBuf::from( &path[4..path.len() - 1]);
            //TODO: startwith can replace equal?
            if crate::INCLUDES.get().unwrap().iter().any(|item| item == &target || target.starts_with(item)) {
                return ReplaceDirResult::IncludesDir;
            }
            else {
                let generated = std::path::Path::new(crate::GENERATEDDIR.get().unwrap());
                if generated.is_absolute() {
                    let modified: Option<String> = Model::fetch_local_replica_sources_dir();
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
            return ReplaceDirResult::NoMatch;
        }
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn load_replica_test() {
        let path = r"\??\E:\TestFuture\ZLMediaKit\3rdpart\media-server\libmov\include\mov-udta.h";
        println!("original: {}", path);
        let index = path.rfind(r"\").unwrap();
        println!("modified: {}", path[4..index + 1].to_string());

        let mut path = std::path::PathBuf::from("D:\\WorkSpace\\OpenSource\\ZLMediaKit\\3rdpart\\media-server\\libmpeg\\include\\mpeg-util.h");
        println!("path stem: {:?}", path.file_stem().unwrap());
        path.set_extension("");
        println!("path new: {:?}", path);
    }
}