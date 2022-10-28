use std::f32::consts::E;


#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct PreSyncFile {
    pub file_kind: std::ffi::OsString,
    pub file_path: std::ffi::OsString,
    pub file_name: std::ffi::OsString,
    pub digest: std::ffi::OsString,
    pub is_exists: bool,
}

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct CompileInput {
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
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
        Self { compiled_filename: filename, compile_status: true, compile_output: std::ffi::OsString::from("") }
    }
}

#[async_trait]
pub trait Compiler: core::marker::Send + core::marker::Sync + 'static {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, compile_input: CompileInput, pool: &tokio::runtime::Handle) -> CompileOutput;
    async fn dist_request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, compile_input: CompileInput, pool: &tokio::runtime::Handle) -> CompileOutput;
}

pub async fn request_compile(compile_input: CompileInput, working_parameters: crate::buildturbo::WorkingParameters, pool: &tokio::runtime::Handle) -> CompileOutput {
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

pub async fn sync_file_to_local(multipart: &mut axum::extract::multipart::Multipart) {

    while let Some(field) = multipart.next_field().await.unwrap() {
        let _name = field.name().expect("fetch name from multipart/form-data failed.").to_string();
        let file_name = field.file_name().expect("fetch file_name from multipart/form-data failed.").to_string();

        let data = field.bytes().await.unwrap();
        let file_path = std::path::Path::new(&file_name);
        let dir = file_path.parent().unwrap();

        if !dir.exists() {
            std::fs::create_dir(dir).unwrap();
        }

        let _ = std::fs::write(file_path, data);
    }
}