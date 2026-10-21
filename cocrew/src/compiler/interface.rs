use crew::compiler::model::{CompilerInput, CompilerOutput, CompiledResults};

pub trait Compiler {
    fn request_compile(&self, compiler_input: CompilerInput) -> (CompilerOutput, Option<CompiledResults>);
}

pub fn cocrew_build(compiler_input: CompilerInput, out_err_stream: crate::compiler::msvc::CompiledResultsStream)
        -> (CompilerOutput, Option<CompiledResults>) {
    let compiler_type = compiler_input.build_and_compiler_type.to_string_lossy();
    let project = compiler_input.project.to_string_lossy().into_owned();

    if compiler_type.contains("msbuild")
        || compiler_type.contains("cmake")
        || compiler_type.contains("clang_cl") {

        let msvc = super::msvc::MSVC {version: "".to_string(), out_err_stream};
        return msvc.request_compile(compiler_input);
    }
    else if compiler_type == "Clang" {
        log::warn!("Clang compiler backend is not implemented yet for project '{}'", project);
        return (CompilerOutput::default(), None);
    }
    else if compiler_type == "GCC" {
        log::warn!("GCC compiler backend is not implemented yet for project '{}'", project);
        return (CompilerOutput::default(), None);
    }
    else {
        log::error!("Unsupported build_and_compiler_type '{}' for project '{}'", compiler_type, project);
        return (CompilerOutput::default(), None);
    }
}
