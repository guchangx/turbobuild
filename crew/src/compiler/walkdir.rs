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
                    if !self.dir.keys().any(|key| path.starts_with(key)) {
                        self.dir.insert(path.clone().into_os_string(), std::sync::Arc::new(FsNode::new()));
                        result.push(path.to_owned());
                    }
                }
            }
        }

        result
    }

    pub fn insnode(&mut self, path: &std::path::Path, is_dir: bool) {

        let mut current = self;

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
        let nodea = self.dir
            .entry(dir.to_owned())
            .or_insert_with(|| std::sync::Arc::new(FsNode::new()));
        let nodem = std::sync::Arc::make_mut(nodea);

        let mut overrides = ignore::overrides::OverrideBuilder::new(dir);
        overrides.add("**/*.{h,hh,hpp,hxx,c,cpp,cxx,cc,dat,inl,ipp,cppm,ixx,h++,inc}").unwrap();

        let overrides = overrides
            .whitelist_no_extension(true)
            .build()
            .unwrap();

        let walker = ignore::WalkBuilder::new(dir)
            .hidden(true) 
            .git_ignore(true)
            .overrides(overrides)
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
                                let _ = tx_.send((p.to_path_buf(), true));
                            }
                        }
                        else {
                            if let Ok(p) = entry.path().strip_prefix(&dir_) {
                                match p.extension() {
                                    Some(ext) => {
                                        if is_tracked_file_extension(ext) {
                                            let _ = tx_.send((p.to_path_buf(), false));
                                        }
                                    }
                                    None => {
                                        let _ = tx_.send((p.to_path_buf(), false));
                                    }
                                }
                            }
                        }
                    }
                    
                    ignore::WalkState::Continue
                })
            });
        });
        
        for (path, is_dir) in rx {
            nodem.insnode(&path, is_dir);
        }

        walk_thread.join().unwrap();
    
    }

    pub fn query<P: AsRef<std::path::Path> + ?Sized>(&self, dir: &P) -> Option<(std::collections::HashSet<std::ffi::OsString>, std::collections::HashSet<std::ffi::OsString>)> {

        if self.dir.is_empty() {
            return None;
        }

        let target = dir.as_ref();

        for (k, v) in self.dir.iter() {
            let Ok(rest) = target.strip_prefix(k) else {
                continue;
            };

            let mut current_ref = v.as_ref();
            let mut matched = true;

            for component in rest.components() {
                match component {
                    std::path::Component::Normal(osstr) => {
                        if let Some(next) = current_ref.dir.get(osstr) {
                            current_ref = next.as_ref();
                        }
                        else {
                            matched = false;
                            break;
                        }
                    }
                    _ => {
                        matched = false;
                        break;
                    }
                }
            }

            if matched {
                return Some((current_ref.dir.keys().cloned().collect(), current_ref.files.iter().cloned().collect()));
            }
        }

        return None;
    }

    pub fn exists<P: AsRef<std::path::Path> + ?Sized>(&self, dir: &P) -> bool {

        if self.dir.is_empty() {
            return false;
        }

        let target = dir.as_ref();

        for (k ,v)  in self.dir.iter() {
            if let Ok(rest) = target.strip_prefix(k) {

                let mut components = rest.components();

                let Some(mut component) = components.next() else {
                    return true;
                };

                let mut current_ref = v.as_ref();

                loop {
                    let osstr = match component {
                        std::path::Component::Normal(c) => c,
                        _ => break,
                    };

                    match components.next() {
                        Some(next_component) => {
                            match current_ref.dir.get(osstr) {
                                Some(next) => {
                                    current_ref = next.as_ref();
                                    component = next_component;
                                }
                                None => break,
                            }
                        }
                        None => {
                            return current_ref.dir.contains_key(osstr) || current_ref.files.contains(osstr);
                        }
                    }
                }
            }
        }
        return false;
    }
}

fn is_tracked_file_extension(ext: &std::ffi::OsStr) -> bool {
    matches!(
        ext.as_encoded_bytes(),
        b"h" | b"hh" | b"hpp" | b"hxx"
            | b"c" | b"cpp" | b"cxx" | b"cc"
            | b"dat" | b"inl" | b"ipp"
            | b"cppm" | b"ixx" | b"h++" | b"inc"
    )
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