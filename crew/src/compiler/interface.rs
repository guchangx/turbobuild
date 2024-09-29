use std::io::Write;


pub trait Compiler {
    fn request_compile(&self, compile_input: crate::compiler::model::CompileInput) 
        -> (crate::compiler::model::CompileOutput, Option<crate::compiler::model::ProcessedResults>);
        
    fn dist_request_compile(&self, compile_input: crate::compiler::model::CompileInput) 
        -> (crate::compiler::model::CompileOutput, Option<crate::compiler::model::ProcessedResults>);
 
    fn remote_request_compile(&self, compile_input: crate::compiler::model::CompileInput) 
        -> (crate::compiler::model::CompileOutput, Option<crate::compiler::model::ProcessedResults>);
}

// local request compile
pub async fn request_compile(compile_input: crate::compiler::model::CompileInput, pool: &tokio::runtime::Handle) 
        -> (crate::compiler::model::CompileOutput, Option<crate::compiler::model::ProcessedResults>) {
                    
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compile_input.build_and_compiler_type.to_string_lossy().contains("CMake") {
        let msvc = crate::compiler::msvc::MSVC {};
        let output = msvc.request_compile(working_parameters, compile_input, pool, grade).await;
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return (crate::compiler::model::CompileOutput::default(), None);
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return (crate::compiler::model::CompileOutput::default(), None);
    }
    else {
        return (crate::compiler::model::CompileOutput::default(), None);
    }
}

// distributed request compile
pub fn dist_request_compile(compile_input: crate::compiler::model::CompileInput, pool: &tokio::runtime::Handle) 
        -> (crate::compiler::model::CompileOutput, Option<crate::compiler::model::ProcessedResults>) {
            
    println!("build and compiler type: {:?}", compile_input.build_and_compiler_type);
    if compile_input.build_and_compiler_type.to_string_lossy().contains("MSBuild")
        || compile_input.build_and_compiler_type.to_string_lossy().contains("CMake")  {
        let msvc = super::msvc::MSVC {};
        let output = msvc.dist_request_compile(working_parameters, compile_input, pool, grade.clone()).await;
        return output;
    }
    else if compile_input.build_and_compiler_type == "Clang" {
        return (crate::compiler::model::CompileOutput::default(), None);
    }
    else if compile_input.build_and_compiler_type == "GCC" {
        return (crate::compiler::model::CompileOutput::default(), None);
    }
    else {
        return (crate::compiler::model::CompileOutput::default(), None);
    }
}