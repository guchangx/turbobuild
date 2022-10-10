
#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct CompileInput {
    pub compiler_path: std::ffi::OsString,
    pub compiler_working_dir: std::ffi::OsString,
    pub compiler_commands: Vec<std::ffi::OsString>,
    pub build_and_compiler_type: std::ffi::OsString,
}

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug, Clone)]
pub struct CompileOutput {
    pub compile_filename : String,
    pub compile_status: bool,
    pub compile_output: String,
}

impl Default for CompileOutput {
    fn default() -> Self {
        Self { compile_filename: "".to_string(), compile_status: true, compile_output: "".to_string() }
    }
}

#[async_trait]
pub trait Compiler: core::marker::Send + core::marker::Sync + 'static {
    async fn request_compile(&self, working_parameters: crate::buildturbo::WorkingParameters, compile_input: CompileInput, pool: &tokio::runtime::Handle) -> CompileOutput;
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