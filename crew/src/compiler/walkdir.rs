//
//D:\WorkSpace\turbobuild\crew\src\compiler\
//walkdir.rs
//msvc.rs
//mod.rs
use compact_str::CompactString;
#[derive(Clone, Default, Debug)]
pub struct FsNode {
    pub dir: rustc_hash::FxHashMap<CompactString, Box<FsNode>>,
    pub files: rustc_hash::FxHashMap<CompactString, u32>, // Stores file sizes in KiB.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Directory,
}

impl FsNode {
    pub fn new() -> Self {
        FsNode {
            dir: rustc_hash::FxHashMap::default(),
            files: rustc_hash::FxHashMap::default(),
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
                        self.dir.insert(CompactString::new(path.to_string_lossy()), Box::new(FsNode::new()));
                        result.push(path.to_owned());
                    }
                }
            }
        }

        result
    }

    pub fn insnode(&mut self, path: &std::path::Path, is_dir: bool, size: u32) {

        let mut current = self;

        let mut components = path.components().filter_map(|component| match component {
            std::path::Component::Normal(osstr) => Some(osstr),
            _ => None,
        }).peekable();

        while let Some(component) = components.next() {
            let should_descend = components.peek().is_some() || is_dir;
            let key = CompactString::new(component.to_string_lossy());

            if should_descend {
                let child = current
                    .dir
                    .entry(key)
                    .or_insert_with(|| Box::new(FsNode::new()));
                current = child.as_mut();
            } else {
                current.files.insert(key, size);
            }
        }
    }

    pub fn add(&mut self, dir: &std::ffi::OsString) {
        let nodea = self.dir
            .entry(CompactString::new(dir.to_string_lossy()))
            .or_insert_with(|| Box::new(FsNode::new()));
        let nodem = nodea.as_mut();

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
        let (tx, rx) = std::sync::mpsc::channel::<(std::path::PathBuf, bool, u32)>();
        let walk_thread = std::thread::spawn(move || {
            walker.run(|| {
                let tx_ = tx.clone();
                let dir_ = dir_.clone();

                Box::new(move |result| {
                    if let Ok(entry) = result {
                        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                            if let Ok(p) = entry.path().strip_prefix(&dir_) {
                                let _ = tx_.send((p.to_path_buf(), true, 0));
                            }
                        }
                        else {
                            if let Ok(p) = entry.path().strip_prefix(&dir_) {
                                match p.extension() {
                                    Some(ext) => {
                                        if is_tracked_file_extension(ext) {
                                            match entry.metadata() {
                                                Ok(metadata) => {
                                                    let _ = tx_.send((
                                                        p.to_path_buf(),
                                                        false,
                                                        file_size_in_kib(metadata.len()),
                                                    ));
                                                }
                                                Err(error) => {
                                                    log::warn!(
                                                        "failed to read metadata for {:?}: {:?}",
                                                        entry.path(),
                                                        error
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    None => {
                                        match entry.metadata() {
                                            Ok(metadata) => {
                                                let _ = tx_.send((
                                                    p.to_path_buf(),
                                                    false,
                                                    file_size_in_kib(metadata.len()),
                                                ));
                                            }
                                            Err(error) => {
                                                log::warn!(
                                                    "failed to read metadata for {:?}: {:?}",
                                                    entry.path(),
                                                    error
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    ignore::WalkState::Continue
                })
            });
        });
        
        for (path, is_dir, size) in rx {
            nodem.insnode(&path, is_dir, size);
        }

        walk_thread.join().unwrap();
    
    }

    pub fn query<P: AsRef<std::path::Path> + ?Sized>(&self, dir: &P) -> Option<(std::collections::HashSet<CompactString>, std::collections::HashSet<CompactString>)> {

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
                        let Some(s) = osstr.to_str() else {
                            matched = false;
                            break;
                        };
                        if let Some(next) = current_ref.dir.get(s) {
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
                return Some((current_ref.dir.keys().cloned().collect(), current_ref.files.keys().cloned().collect()));
            }
        }

        return None;
    }

    pub fn exists<P: AsRef<std::path::Path> + ?Sized>(&self, path: &P, kind: NodeKind) -> bool {

        if self.dir.is_empty() {
            return false;
        }

        let target = path.as_ref();

        for (k ,v)  in self.dir.iter() {
            if let Ok(rest) = target.strip_prefix(k) {

                let mut components = rest.components();

                let Some(mut component) = components.next() else {
                    if kind == NodeKind::Directory {
                        return true;
                    }
                    else {
                        return false;
                    }
                };

                let mut current_ref = v.as_ref();

                loop {
                    let osstr = match component {
                        std::path::Component::Normal(c) => c,
                        _ => break,
                    };

                    let Some(s) = osstr.to_str() else {
                        break;
                    };

                    match components.next() {
                        Some(next_component) => {
                            match current_ref.dir.get(s) {
                                Some(next) => {
                                    current_ref = next.as_ref();
                                    component = next_component;
                                }
                                None => break,
                            }
                        }
                        None => {
                            if kind == NodeKind::Directory {
                                return current_ref.dir.contains_key(s);
                            }
                            else if kind == NodeKind::File {
                                return current_ref.files.contains_key(s);
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    pub fn sourcefile_size<P: AsRef<std::path::Path> + ?Sized>(&self, path: &P) -> Option<u32> {
        let target = path.as_ref();

        for (root, node) in &self.dir {
            let Ok(rest) = target.strip_prefix(root) else {
                continue;
            };

            let mut current = node.as_ref();
            let mut components = rest.components().peekable();

            while let Some(component) = components.next() {
                let std::path::Component::Normal(name) = component else {
                    return None;
                };

                let Some(name) = name.to_str() else {
                    return None;
                };

                if components.peek().is_none() {
                    return current.files.get(name).copied();
                }

                current = current.dir.get(name)?.as_ref();
            }
        }

        None
    }
    pub fn shrink(&mut self) {
        self.dir.shrink_to_fit();
        self.files.shrink_to_fit();

        for entry in self.dir.values_mut() {
            entry.shrink();
        }
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

pub fn file_size_in_kib(size_in_bytes: u64) -> u32 {
    let size_in_kib = size_in_bytes
        .saturating_add(1023)
        .checked_div(1024)
        .unwrap_or(u64::MAX);

    u32::try_from(size_in_kib).unwrap_or(u32::MAX)
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
fn exists_file_or_dir_test() {
    let now = std::time::Instant::now();
    let path = std::env::current_dir().unwrap();
    println!("Current path: {:?}", path);
    let mut fs = FsNode::new();
    fs.add(&path.clone().into_os_string());
    println!("query_fs_node_test_elapsed: {:?}", now.elapsed());
    println!("query_fs_node_test_node: {:#?}", fs);

    let files = fs.exists(&path, NodeKind::File);
    println!("exists fs node dirs: {:?}", files);

    let files = fs.exists(&path, NodeKind::Directory);
    println!("exists fs node dirs: {:?}", files);


    let files = fs.exists(&path.join("Cargo.toml"), NodeKind::File);
    println!("exists fs node files: {:?}", files);

    let files = fs.exists(&path.join("Cargo.toml"), NodeKind::Directory);
    println!("exists fs node files: {:?}", files);
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