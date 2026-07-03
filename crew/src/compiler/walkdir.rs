//
//D:\WorkSpace\turbobuild\crew\src\compiler\
//walkdir.rs
//msvc.rs
//mod.rs

pub struct WalkDir {
    pub dirfiles: dashmap::DashMap<String, std::sync::Arc<std::collections::HashSet<String>>>,
}

impl WalkDir {
    pub fn new() -> Self {
        WalkDir {
            dirfiles: dashmap::DashMap::new(),
        }
    }

    pub fn add(&self, dirs: &Vec<std::path::PathBuf>) -> std::vec::Vec::<(String, std::sync::Arc<std::collections::HashSet<String>>)> {

        let mut result = std::vec::Vec::<(String, std::sync::Arc<std::collections::HashSet<String>>)>::new();

        for dir in dirs {
            let key = dir.to_str().unwrap().to_string();

            if let Some(v) = self.dirfiles.get(&key) {
                result.push((key, v.value().clone()));
                continue;
            }

            let files = crate::compiler::dependency::query_include_dir(&std::vec![dir.to_owned()]);
            for (key, value) in files {
                let value = std::sync::Arc::new(value);
                self.dirfiles.insert(key.clone(), std::sync::Arc::clone(&value));
                result.push((key, value));
            }
        }

        result
    }


    pub fn clear(&self) {
        self.dirfiles.clear();
    }
}