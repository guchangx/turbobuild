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
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
}
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct PrecompiledSource {
    pub contents: Option<Vec<u8>>,
    pub path: std::ffi::OsString, 
}
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Default)]
pub struct CompilerOutput {
    pub filename: Vec<std::ffi::OsString>,
    pub status: bool,
    pub output: std::ffi::OsString,
}

impl CompilerOutput {
    pub fn set(&mut self, value: Self) {
        self.filename = value.filename;
        self.status = value.status;
        self.output = value.output;
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct ProcessedResult {
    pub source_file: std::ffi::OsString,
    pub obj: Option<(std::ffi::OsString, Vec<u8>)>,
    pub pdb: Option<(std::ffi::OsString, Vec<u8>)>,
    pub idb: Option<(std::ffi::OsString, Vec<u8>)>,
}

pub type ProcessedResults = Vec<ProcessedResult>;