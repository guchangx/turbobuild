
use crate::compiler::model::{CompilerInput, CompilerOutput, ProcessedResults};

pub trait Compiler {
    fn request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<ProcessedResults>);
}

// local request compile
pub async fn request_compile(compiler_input: CompilerInput, runtime: std::sync::Arc<tokio::runtime::Handle>, 
                            packager: std::sync::Arc<std::sync::Mutex::<crate::communicate::distributor::Distributor>>) 
                            -> (CompilerOutput, Option<ProcessedResults>) {
                    
    if compiler_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
            
        let msvc = crate::compiler::msvc::MSVC {
            working_parameters: crate::platform::windows::WindowsCompilerEnv::default(),
            pool: runtime,
            sender: packager,
        };
           
        let output = msvc.request_compile(compiler_input);
        return output;
    }
    else if compiler_input.build_and_compiler_type == "Clang" {
        return (CompilerOutput::default(), None);
    }
    else if compiler_input.build_and_compiler_type == "GCC" {
        return (CompilerOutput::default(), None); 
    }
    else {
        return (CompilerOutput::default(), None);
    }
}