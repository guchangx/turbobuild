
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct SyncData {
    pub sync_kind: std::ffi::OsString,
    pub toolchain_path: std::ffi::OsString,
    pub windows_kits_path: std::ffi::OsString,
    pub file_path: std::ffi::OsString,
    pub file_name: std::ffi::OsString,
    pub digest: std::ffi::OsString,
    pub is_exists: bool,
}

impl Default for SyncData {
    fn default() -> Self {
        Self {
            sync_kind: std::ffi::OsString::new(),
            toolchain_path: std::ffi::OsString::new(),
            windows_kits_path: std::ffi::OsString::new(),
            file_path: std::ffi::OsString::new(),
            file_name: std::ffi::OsString::new(),
            digest: std::ffi::OsString::new(),
            is_exists: false,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default)]
pub struct CompilerInput {
    pub solution: std::ffi::OsString,
    pub project: std::ffi::OsString,
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
    pub envs: std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct PrecompiledSource {
    pub contents: Option<Vec<u8>>,
    pub path: std::ffi::OsString, 
}

#[derive(Debug, Clone, Default)]
pub struct CompilerOutput {
    pub status: u32,
    pub out: std::sync::Arc<Vec<u8>>, 
    pub err: std::sync::Arc<Vec<u8>>, 
}

impl CompilerOutput {
    pub fn set(&mut self, value: Self) {
        self.status = value.status;
        self.out = value.out;
        self.err = value.err;
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CompiledResult {
    pub source_file: std::ffi::OsString,
    pub obj: Option<(std::ffi::OsString, Vec<u8>)>,
    pub pdb: Option<(std::ffi::OsString, Vec<u8>)>,
    pub idb: Option<(std::ffi::OsString, Vec<u8>)>,
}

pub type CompiledResults = Vec<CompiledResult>;


pub type OutputCallback = std::sync::Arc<dyn Fn(crate::compiler::model::CompilerOutput) 
            -> Box<dyn std::future::Future<Output = ()> + Send> 
            + Send + Sync>;

pub static WALK_DIRS_FILES: std::sync::LazyLock<crate::compiler::walkdir::WalkDir> = std::sync::LazyLock::new(|| {
    crate::compiler::walkdir::WalkDir::new()
});

pub static WALK_FS_NODE: std::sync::LazyLock<std::sync::Arc<crate::compiler::walkdir::FsNode>> = std::sync::LazyLock::new(|| {
    std::sync::Arc::new(crate::compiler::walkdir::FsNode::new())
});