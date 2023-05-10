
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
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
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct EnvInput {
    pub winkits_includes_path: Vec<std::ffi::OsString>,
    pub compiler_path: std::ffi::OsString,
    pub msvc_includes_path: std::ffi::OsString,
    pub msvc_version: std::ffi::OsString,
    pub env_args: std::ffi::OsString,
}
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct CompileInput {
    pub compiler_path_or_arch: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,

    pub env_input: Option<EnvInput>,
}
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct PrecompiledSource {
    pub preprocessed_source_contents: Option<Vec<u8>>,
    pub preprocessed_source_path: std::ffi::OsString, 
}
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct CompileOutput {
    pub compiled_filename: Vec<std::ffi::OsString>,
    pub compile_status: bool,
    pub compile_output: std::ffi::OsString,
}

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct ProcessedResult {
    pub source_file: std::ffi::OsString,
    pub obj: Option<(std::ffi::OsString, Vec<u8>)>,
    pub pdb: Option<(std::ffi::OsString, Vec<u8>)>,
    pub idb: Option<(std::ffi::OsString, Vec<u8>)>,
}

pub type ProcessedResults = Vec<ProcessedResult>;

impl Default for CompileOutput {
    fn default() -> Self {
        let filename: Vec<std::ffi::OsString> = Vec::new();
        Self { 
            compiled_filename: filename, 
            compile_status: false, 
            compile_output: std::ffi::OsString::new(), 
        }
    }
}

impl CompileOutput {
    pub fn set(&mut self, value: Self) {
        self.compiled_filename = value.compiled_filename;
        self.compile_status = value.compile_status;
        self.compile_output = value.compile_output;
    }
}

#[async_trait]
pub trait Compiler: core::marker::Send + core::marker::Sync + 'static {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                compile_input: CompileInput, pool: &tokio::runtime::Handle,
                grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>) -> (CompileOutput, Option<ProcessedResults>);
    async fn dist_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                compile_input: CompileInput, pool: &tokio::runtime::Handle, 
                grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>) -> (CompileOutput, Option<ProcessedResults>);
    async fn remote_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                compile_input: CompileInput, pool: &tokio::runtime::Handle, 
                grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>) -> (CompileOutput, Option<ProcessedResults>);
}

pub async fn request_compile(working_parameters: crate::buildturbo::WorkingParameters, compile_input: CompileInput, 
                pool: &tokio::runtime::Handle, 
                grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>) -> (CompileOutput, Option<super::compiler::ProcessedResults>) {
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compile_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
        let msvc = super::msvc::MSVC {};
        let output = msvc.request_compile(working_parameters, compile_input, pool, grade).await;
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return (CompileOutput::default(), None);
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return (CompileOutput::default(), None);
    }
    else {
        return (CompileOutput::default(), None);
    }
}

pub async fn dist_request_compile(working_parameters: crate::buildturbo::WorkingParameters, compile_input: CompileInput, 
    pool: &tokio::runtime::Handle,
    grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>) -> (CompileOutput, Option<super::compiler::ProcessedResults>) {
    println!("build and compiler type: {:?}", compile_input.build_and_compiler_type);
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compile_input.build_and_compiler_type.to_string_lossy().contains("CMake")  {
        let msvc = super::msvc::MSVC {};
        let output = msvc.dist_request_compile(working_parameters, compile_input, pool, grade.clone()).await;
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return (CompileOutput::default(), None);
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return (CompileOutput::default(), None);
    }
    else {
        return (CompileOutput::default(), None);
    }
}

pub async fn remote_request_compile(multipart: &mut axum::extract::multipart::Multipart, 
        working_parameters: crate::buildturbo::WorkingParameters, pool: &tokio::runtime::Handle, 
        grade: std::sync::Arc<std::sync::Mutex<crate::utils::grade::LocalGrade>>) -> (CompileOutput, Option<ProcessedResults>)
{
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.name() {
            if name.cmp("precompiled_source") == std::cmp::Ordering::Equal {
                let file_path = field.file_name().expect("fetch file_name from multipart/form-data failed.");
                log::trace!("remote request copmile sync file name: {:?}", file_path);
                if !file_path.is_empty() {
                    let path = std::path::PathBuf::from(file_path);
                    let dir = path.parent().unwrap();
                    if !dir.exists() {
                        match std::fs::create_dir_all(dir) {
                            Ok(_) => {},
                            Err(error) => {
                                log::warn!("dist worker create .i file dir {:?} failed. {:?}.", dir, error);
                            },
                        }
                    }
                    
                    if let Ok(contents) = field.bytes().await {
                        match std::fs::write(path, contents) {
                            Ok(_) => {},
                            Err(error) => {
                                log::warn!("sync precompiled source .i file failed. {:?}", error);
                            },
                        }
                    }
                }
            }
            else if name.cmp("compile_input") == std::cmp::Ordering::Equal {
                if let Ok(contents) = field.bytes().await {
                    if contents.is_empty() {
                        return (CompileOutput::default(), None);
                    }
                    else {
                        let contents = contents.to_vec();
                        let contents = std::str::from_utf8(&contents).unwrap();
                        let input:CompileInput = serde_json::from_str(&contents).expect("deserialize compileiput failed.");
                        if input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
                            || input.build_and_compiler_type.to_string_lossy().contains("CMake")  {
                            let msvc = super::msvc::MSVC {};
                            let (output, results) = msvc.remote_request_compile(working_parameters, input, pool, grade.clone()).await;
                            return (output, results);
                        }
                        else if input.build_and_compiler_type == "Clang" {
                            return (CompileOutput::default(), None);
                        }
                        else if input.build_and_compiler_type == "GCC" {
                            return (CompileOutput::default(), None);
                        }
                        else {
                            return (CompileOutput::default(), None);
                        }
                    }
                }
            }
            else {
                log::warn!("multipart field name: {:?} do not match.", name);
            }
        }
    }
    return (CompileOutput::default(), None);
}