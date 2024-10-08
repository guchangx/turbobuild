
use crew::compiler::model::{CompilerInput, CompilerOutput, ProcessedResults};

pub trait Compiler {

    fn dist_request_compile(&self, compile_input: CompilerInput) 
        -> (CompilerOutput, Option<ProcessedResults>);
}

pub fn build(compile_input: CompilerInput)
        -> (CompilerOutput, Option<ProcessedResults>) {
            
    println!("build and compiler type: {:?}", compile_input.build_and_compiler_type);
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compile_input.build_and_compiler_type.to_string_lossy().contains("CMake")  {
        let msvc = super::msvc::MSVC {version: "".to_string()};
        let output = msvc.dist_request_compile(compile_input);
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return (CompilerOutput::default(), None);
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return (CompilerOutput::default(), None);
    }
    else {
        return (CompilerOutput::default(), None);
    }
}