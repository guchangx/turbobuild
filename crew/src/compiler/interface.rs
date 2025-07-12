
use crate::compiler::model::{CompilerInput, CompilerOutput};

pub trait Compiler {
    fn request_compile(&self, compiler_input: CompilerInput) -> CompilerOutput;
}

// local request compile
pub async fn request_compile(compiler_input: CompilerInput, runtime: std::sync::Arc<tokio::runtime::Handle>, env: Option<crate::platform::windows::WindowsCompilerEnv>,
                            distor: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>) 
                            -> CompilerOutput {
                    
    if compiler_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
            
        let msvc = crate::compiler::msvc::MSVC {
            work_env: if env.is_some() { env.unwrap() } else { crate::platform::windows::WindowsCompilerEnv::default() },
            runtime: runtime,
            sender: distor,
        };
        let output = msvc.request_compile(compiler_input);
        return output;
    }
    else if compiler_input.build_and_compiler_type == "Clang" {
        return CompilerOutput::default();
    }
    else if compiler_input.build_and_compiler_type == "GCC" {
        return CompilerOutput::default(); 
    }
    else {
        return CompilerOutput::default();
    }
}