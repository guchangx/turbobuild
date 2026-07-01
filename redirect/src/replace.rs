use crate::DEPENDENCYS;


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
        if extension == "nls" {
            return ReplaceResult::NoMatch;
        }
        if extension == "h" || extension == "hpp"  || extension == "inl" {
            return ReplaceResult::NoMatch;
        }
        else if extension == "pdb"  || extension == "idb" {
            if let Some(replica_pdbpath) = crate::REPLICA_PDBPATH.get() {
                let mut pdb = replica_pdbpath.to_string();

                if extension == "idb" {
                    pdb = pdb.replace(".pdb", ".idb");
                }

                *path = pdb;
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
pub enum ReplaceNtResult {
    Success,
    NoMatch,
    IncludesDir,
    VirtualIncludesDir(String),
    FilePath,
    NeedObtain(String),
}

#[derive(PartialEq, Debug)]
pub enum ReplaceType {
    Unknown,
    File,
    Dir,
}


pub fn nt_replace(path: &mut String, rtype: ReplaceType) -> ReplaceNtResult {

    if rtype == ReplaceType::File {
        if path.contains(r"AppData\Local\Temp\") {
            return ReplaceNtResult::NoMatch;
        }
        else if let Some(extension) = std::path::Path::new(path).extension() {
            if extension == "h" || extension == "hpp" || extension == "hxx" || extension == "inl" || extension == "inc" || extension == "ipp" {
                if path.contains(r"Replica\MSVC") || path.contains(r"Replica\Windows Kits") {
                    return ReplaceNtResult::FilePath;
                }
                else if DEPENDENCYS.get().map_or(false, |deps| {
                    deps.iter().any(|(key, value)| {
                        if path.contains(key) {
                            let modified = format!(r"\??\{}", value);
                            *path = modified;
                            return true;
                        }
                        return false;
                    })
                }) {
                    return ReplaceNtResult::Success;
                }
                else if path[4..].starts_with(crate::REPLICADIR.get().unwrap_or(&"*".to_string())) {
                    return ReplaceNtResult::FilePath;
                }
                else {
                    if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                        Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::Success, |modified| {
                            let unmodified = path.to_string();
                            *path = modified;
                            return ReplaceNtResult::NeedObtain(unmodified);
                        })
                    }
                    else {
                        let index = path.find(":\\");
                        if let Some(i) = index {
                            let dir = &path[i + 2..];
                            let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&dir).to_string_lossy().to_string();

                            let modified = format!(r"\??\{}", modified);                 
                            *path = modified;
                            return ReplaceNtResult::Success;
                        }
                        else {
                            let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&path).to_string_lossy().to_string();
                            let modified = format!(r"\??\{}", modified);
                            let unmodified = path.to_string();
                            *path = modified;
                            return ReplaceNtResult::NeedObtain(unmodified);
                        }
                    }
                }
            }
            else if extension == "cpp" || extension == "cxx" || extension == "cc" || extension == "c" || extension == "c++" {
                if path[4..].starts_with(crate::REPLICADIR.get().unwrap_or(&"*".to_string())) {
                    return ReplaceNtResult::FilePath;
                }
                else if path.contains("moc_") || path.contains("mocs_") || path.contains("qrc_") {
                    if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                        Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::Success, |modified| {
                            let unmodified = path.to_string();
                            *path = modified;        
                            return ReplaceNtResult::NeedObtain(unmodified);
                        })
                    }
                    else {
                        if let Some(name) = std::path::Path::new(path).file_name() {
                            let modified = std::path::Path::new(&*crate::WORKINGDIR).join(crate::GENERATEDDIR.get().unwrap()).join(&name).to_string_lossy().to_string();
                            let modified = format!(r"\??\{}", modified);
                            let unmodified = path.to_string();
                            *path = modified;
                            return ReplaceNtResult::NeedObtain(unmodified);
                        }
                        else {
                            return ReplaceNtResult::FilePath;
                        }
                    }
                }
                else {
                    if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                        let filestem = std::path::Path::new(&path[4..]).file_stem().unwrap();
                        let any = crate::SOURCES.get().unwrap().iter().any(|item| {
                            if filestem == std::path::Path::new(item).file_stem().unwrap() {
                                return true;
                            } 
                            else {
                                return false;
                            }
                        });

                        if any {
                            Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::Success, |modified| {
                                *path = modified;
                                return ReplaceNtResult::Success;
                            })
                        }
                        else {
                            crate::log!(warn, "source file not in sources dir, need obtain: {}", path);
                            Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::Success, |modified| {
                                let unmodified = path.to_string();
                                *path = modified;
                                return ReplaceNtResult::NeedObtain(unmodified);
                            })
                        }
                    }
                    else {
                        let index = path.find(":\\");
                        if let Some(i) = index {
                            let dir = &path[i + 2..];
                            let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&dir).to_string_lossy().to_string();

                            let modified = format!(r"\??\{}", modified);                 
                            *path = modified;
                            return ReplaceNtResult::Success;
                        }
                        else {
                            return ReplaceNtResult::FilePath;
                        }
                    }
                }
            }
            else if extension == "dat" {
                if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                    Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::Success, |modified| {
                        let unmodified = path.to_string();
                        *path = modified;        
                        return ReplaceNtResult::NeedObtain(unmodified);
                    })
                }
                else {
                    let index = path.find(":\\");
                    if let Some(i) = index 
                    {
                        let dir = &path[i + 2..];
                        let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&dir).to_string_lossy().to_string();
                        let modified = format!(r"\??\{}", modified);
                        let unmodified = path.to_string();
                        *path = modified;
                        return ReplaceNtResult::NeedObtain(unmodified);
                    }
                    else {
                        let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&path).to_string_lossy().to_string();
                        let modified = format!(r"\??\{}", modified);
                        let unmodified = path.to_string();
                        *path = modified;
                        return ReplaceNtResult::NeedObtain(unmodified);
                    }
                }
            }
            else {
                //.pdb .idb .obj .nls
                return ReplaceNtResult::NoMatch;
            }
        }
        else {
            if path[4..].starts_with(crate::REPLICADIR.get().unwrap_or(&"*".to_string())) {
                return ReplaceNtResult::FilePath;
            }
            else {
                if path.contains(crate::SOLUTIONNAME.get().unwrap()) {
                    Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::Success, |modified| {
                        let unmodified = path.to_string();
                        *path = modified;
                        return ReplaceNtResult::NeedObtain(unmodified);
                    })
                }
                else {
                    let index = path.find(":\\");
                    if let Some(i) = index 
                    {
                        let dir = &path[i + 2..];
                        let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&dir).to_string_lossy().to_string();
                        let modified = format!(r"\??\{}", modified);
                        let unmodified = path.to_string();
                        *path = modified;
                        return ReplaceNtResult::NeedObtain(unmodified);
                    }
                    else {
                        let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&path).to_string_lossy().to_string();
                        let modified = format!(r"\??\{}", modified);
                        let unmodified = path.to_string();
                        *path = modified;
                        return ReplaceNtResult::NeedObtain(unmodified);
                    }
                }
            }
        }
    }
    else if rtype == ReplaceType::Dir {
        if path.contains(r"Replica\MSVC") || path.contains(r"Replica\Windows Kits") {
            return ReplaceNtResult::NoMatch;
        }
        else if crate::SOLUTIONNAME.get().is_some() && path.contains(crate::SOLUTIONNAME.get().unwrap()) {

            let target = std::path::PathBuf::from(&path[4..path.len() - 1]);
            if crate::INCLUDES.get().unwrap().iter().any(|item| item == &target || target.starts_with(item)) {
                return Model::fetch_local_replica_project_path(&path).map_or(ReplaceNtResult::VirtualIncludesDir(path.to_string()), |modified| {
                    let unmodified = path.to_string();
                    *path = modified;
                    ReplaceNtResult::VirtualIncludesDir(unmodified)
                });
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
                        return ReplaceNtResult::Success;
                    } 
                    else {
                        crate::log!(warn, "replace sources dir is not ready, original: {}", path);
                        return ReplaceNtResult::NoMatch;
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
                    return ReplaceNtResult::Success;
                }
            }
        }
        else {
            let target = std::path::PathBuf::from(&path[4..path.len() - 1]);
            if crate::INCLUDES.get().unwrap().iter().any(|item| item == &target || target.starts_with(item)) {

                let index = path.find(":\\");
                if let Some(i) = index {
                    let dir = &path[i + 2..];
                    let modified = std::path::Path::new(crate::REPLICADIR.get().unwrap()).join(&dir).to_string_lossy().to_string();
                    let unmodified = path.to_string();
                    let modified = format!(r"\??\{}", modified);
                    
                    *path = modified;
                    return ReplaceNtResult::VirtualIncludesDir(unmodified);
                }
                else {
                    crate::log!(warn, "replace extern dir is not ready, original: {}", path);
                    return ReplaceNtResult::NoMatch;
                } 
            }
            return ReplaceNtResult::NoMatch;
        }
    }
    else {
        return ReplaceNtResult::NoMatch;
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