
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
pub struct CompileInput {
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
    pub env_input: Option<EnvInput>,
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
pub struct CompileOutput {
    pub compiled_filename: Vec<std::ffi::OsString>,
    pub compile_status: bool,
    pub compile_output: std::ffi::OsString,
}

impl Default for CompileOutput {
    fn default() -> Self {
        let filename: Vec<std::ffi::OsString> = Vec::new();
        Self { compiled_filename: filename, compile_status: true, compile_output: std::ffi::OsString::new() }
    }
}

#[async_trait]
pub trait Compiler: core::marker::Send + core::marker::Sync + 'static {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                compile_input: CompileInput, pool: &tokio::runtime::Handle) -> CompileOutput;
    async fn dist_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, 
                compiler_env: crate::platform::windows::WindowsCompilerEnv, compile_input: CompileInput, pool: &tokio::runtime::Handle) -> CompileOutput;
}

pub async fn request_compile(working_parameters: crate::buildturbo::WorkingParameters, compile_input: CompileInput, pool: &tokio::runtime::Handle) -> CompileOutput {
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSVC") {
        let msvc = super::msvc::MSVC {};
        let output = msvc.request_compile(working_parameters, compile_input, pool).await;
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return CompileOutput::default();
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return CompileOutput::default();
    }
    else {
        return CompileOutput::default();
    }
}

pub async fn dist_request_compile(working_parameters: crate::buildturbo::WorkingParameters, compiler_env: crate::platform::windows::WindowsCompilerEnv, compile_input: CompileInput,   pool: &tokio::runtime::Handle) -> CompileOutput {
        if compile_input.build_and_compiler_type.to_string_lossy().contains("MSVC") {
        let msvc = super::msvc::MSVC {};
        let output = msvc.dist_request_compile(working_parameters, compiler_env, compile_input, pool).await;
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return CompileOutput::default();
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return CompileOutput::default();
    }
    else {
        return CompileOutput::default();
    }
}