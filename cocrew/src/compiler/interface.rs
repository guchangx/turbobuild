
use crew::compiler::model::{CompilerInput, CompilerOutput, CompiledResults};

pub trait Compiler {
    fn request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<CompiledResults>);
}

pub fn build(compiler_input: CompilerInput)
        -> (CompilerOutput, Option<CompiledResults>) {
            
    log::info!("build and compiler type: {:?}", compiler_input.build_and_compiler_type);
    if compiler_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compiler_input.build_and_compiler_type.to_string_lossy().contains("CMake")  {

        let msvc = super::msvc::MSVC {version: "".to_string()};
        
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