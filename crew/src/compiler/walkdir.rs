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

    fn insfile(&mut self, path: &std::path::Path, is_dir: bool) {
        let mut current = self;
        let mut components = path.components().filter_map(|component| match component {
            std::path::Component::Normal(osstr) => Some(osstr.to_owned()),
            _ => None,
        }).peekable();

        while let Some(component) = components.next() {
            if components.peek().is_some() {
                let child = current
                    .dir
                    .entry(component)
                    .or_insert_with(|| std::sync::Arc::new(FsNode::new()));
                current = std::sync::Arc::make_mut(child);
            }
            else {
                if is_dir {
                    let child = current
                        .dir
                        .entry(component)
                        .or_insert_with(|| std::sync::Arc::new(FsNode::new()));
                    current = std::sync::Arc::make_mut(child);
                } else {
                    current.files.insert(component);
                }
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
                                p.extension().map(|ext| ext.to_string_lossy()).map(|ext| {
                                    if ext == "h" || ext == "hpp" || ext == "c" || ext == "cpp" || ext == "cc" {
                                        println!("Found file: {} at depth {}", p.display(), entry.depth());
                                        let _ = tx_.send((p.to_path_buf(), false));
                                    }
                                });
                            }
                        }
                    }
                    
                    ignore::WalkState::Continue
                })
            });
        });
        
        for (path, is_dir) in rx {
            self.insfile(&path, is_dir);
        }

        println!("Finished walking directory: {:#?}", self.dir);
        println!("Finished walking directory: {:#?}", self.files);

        walk_thread.join().unwrap();
    
    }

    pub fn query(&self, dir: &std::path::Path) -> Option<std::sync::Arc<FsNode>> {
        let mut current_node = None;
        let mut current_ref = self;
        for component in dir.components() {
            match component {
                std::path::Component::Normal(os_str) => {
                    if let Some(next) = current_ref.dir.get(os_str) {
                        current_ref = next.as_ref();
                        current_node = Some(next.clone());
                    } else if current_ref.files.contains(os_str) {
                        return None;
                    } else {
                        return None;
                    }
                }
                _ => {}
            }
        }
        current_node
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
    FsNode::new().add(&std::ffi::OsString::from("D:\\WorkSpace\\OpenSource\\ZLMediaKit\\3rdpart\\media-server\\libmov"));
    println!("query_include_dir_ignore_test: {:?}", now.elapsed());
}