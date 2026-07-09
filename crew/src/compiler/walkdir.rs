//
//D:\WorkSpace\turbobuild\crew\src\compiler\
//walkdir.rs
//msvc.rs
//mod.rs

#[derive(Clone, Default, Debug)]
pub struct FsNode {
    pub dir: rustc_hash::FxHashMap<std::ffi::OsString, std::sync::Arc<FsNode>>,
    pub files: rustc_hash::FxHashSet<std::ffi::OsString>,
}

impl FsNode {
    pub fn new() -> Self {
        FsNode {
            dir: rustc_hash::FxHashMap::default(),
            files: rustc_hash::FxHashSet::default(),
        }
    }

    pub fn roots(&mut self, dirs: &mut Vec<std::path::PathBuf>) -> Vec<std::path::PathBuf> {
        dirs.sort();

        let mut result = Vec::new();

        for path in dirs {
            match result.last() {
                Some(parent) if path.starts_with(parent) => {
                   
                }
                _ => {
                    if !self.dir.contains_key(&path.clone().into_os_string()) {
                        self.dir.insert(path.clone().into_os_string(), std::sync::Arc::new(FsNode::new()));
                        result.push(path.to_owned());
                    }
                }
            }
        }

        result
    }

    fn insnode(&mut self, path: &std::path::Path, is_dir: bool) {
        let v = self.dir.iter_mut().next().unwrap().1;
        let mut current = std::sync::Arc::make_mut(v);

        let mut components = path.components().filter_map(|component| match component {
            std::path::Component::Normal(osstr) => Some(osstr),
            _ => None,
        }).peekable();

        while let Some(component) = components.next() {
            let should_descend = components.peek().is_some() || is_dir;

            if should_descend {
                let child = current
                    .dir
                    .entry(component.to_owned())
                    .or_insert_with(|| std::sync::Arc::new(FsNode::new()));
                current = std::sync::Arc::make_mut(child);
            } else {
                current.files.insert(component.to_owned());
            }
        }
    }

    pub fn add(&mut self, dir: &std::ffi::OsString) {
        self.dir.insert(dir.to_owned(), std::sync::Arc::new(FsNode::new()));

        let walker = ignore::WalkBuilder::new(dir)
            .hidden(true) 
            .git_ignore(false)
            .build_parallel();

        let dir_ = dir.clone();
        let (tx, rx) = std::sync::mpsc::channel::<(std::path::PathBuf, bool)>();
        let walk_thread = std::thread::spawn(move || {
            walker.run(|| {
                let tx_ = tx.clone();
                let dir_ = dir_.clone();

                Box::new(move |result| {
                    if let Ok(entry) = result {

                        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                            if let Ok(p) = entry.path().strip_prefix(&dir_) {
                                println!("Found dir: {} at depth {}", p.display(), entry.depth());
                                let _ = tx_.send((p.to_path_buf(), true));
                            }
                        }
                        else {
                            if let Ok(p) = entry.path().strip_prefix(&dir_) {
                                p.extension().map(|ext| 
                                    if ext == "h" || ext == "hpp" || ext == "c" || ext == "cpp" || ext == "cc" {
                                        println!("Found file: {} at depth {}", p.display(), entry.depth());
                                        let _ = tx_.send((p.to_path_buf(), false));
                                    }
                                );
                            }
                        }
                    }
                    
                    ignore::WalkState::Continue
                })
            });
        });
        
        for (path, is_dir) in rx {
            self.insnode(&path, is_dir);
        }

        walk_thread.join().unwrap();
    
    }

    pub fn query<P: AsRef<std::path::Path>>(&self, dir: &P) -> Option<(std::collections::HashSet<std::ffi::OsString>, std::collections::HashSet<std::ffi::OsString>)> {

        if self.dir.is_empty() {
            return None;
        }

        let (k ,v) = self.dir.iter().next().unwrap();
        
        if let Ok(dir) = dir.as_ref().strip_prefix(k) {

            let mut current_ref = v.as_ref();
            for component in dir.components() {
                match component {
                    std::path::Component::Normal(osstr) => {
                        if let Some(next) = current_ref.dir.get(osstr) {
                            current_ref = next.as_ref();
                        }
                        else {
                            return None;
                        }
                    }
                    _ => {
                        return None;
                    }
                }
            }
            return Some((current_ref.dir.keys().cloned().collect(), current_ref.files.iter().cloned().collect()));
        }
        else {
            return None;
        }
    }

    pub fn exists<P: AsRef<std::path::Path>>(&self, dir: &P) -> bool {

        if self.dir.is_empty() {
            return false;
        }

        let (k ,v) = self.dir.iter().next().unwrap();

        if let Ok(dir) = dir.as_ref().strip_prefix(k) {

            let mut current_ref = v.as_ref();
            for component in dir.components() {
                match component {
                    std::path::Component::Normal(osstr) => {
                        if let Some(next) = current_ref.dir.get(osstr) {
                            current_ref = next.as_ref();
                        }
                        else if current_ref.files.contains(osstr) {
                            return true;
                        }
                        else {
                            return false;
                        }
                    }
                    _ => {
                        return false;
                    }
                }
            }
            return false;
        }
        else {
            return false;
        }
    }
}



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
                self.dirfiles.insert(key.clone(), value.clone());
                result.push((key, value));
            }
        }
        result
    }

    pub fn exists(&self, dir: &str) -> bool {
        self.dirfiles.contains_key(dir)
    }

    pub fn clear(&self) {
        self.dirfiles.clear();
    }
}

#[test]
fn query_include_dir_ignore_test() {
    let now = std::time::Instant::now();
    let path = std::env::current_dir().unwrap();
    println!("Current path: {:?}", path);
    let mut fs = FsNode::new();
    fs.add(&path.into_os_string());
    println!("query_fs_node_test_elapsed: {:?}", now.elapsed());
    println!("query_fs_node_test_node: {:#?}", fs);

    let files = fs.query(&std::path::Path::new("G:\\turbobuild\\crew\\src"));
    println!("query fs node dirs and files: {:?}", files);
}

#[test]
fn strip_prefix_test() {
    let path = std::path::Path::new("C:\\Users\\user\\Documents\\project\\src");
    let prefix = std::path::Path::new("C:\\Users\\Documents");
    if let Ok(stripped) = path.strip_prefix(prefix) {
        println!("Stripped path: {:?}", stripped);
    } else {
        println!("Failed to strip prefix");
    }

    let path = std::path::Path::new("C:\\Users\\user\\Documents\\project\\src");
    let prefix = std::path::Path::new("C:\\Users\\Documents");
    if let Ok(stripped) = path.strip_prefix(prefix) {
        println!("Stripped path: {:?}", stripped);
    } else {
        println!("Failed to strip prefix");
    }

    let path = std::path::Path::new("C:\\Users\\user\\Documents\\project\\src");
    let prefix = std::path::Path::new("C:\\Users\\project");
    if let Ok(stripped) = path.strip_prefix(prefix) {
        println!("Stripped path: {:?}", stripped);
    } else {
        println!("Failed to strip prefix");
    }
}