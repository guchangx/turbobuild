use std::io::{Read, Write};

#[derive(Default, Clone)]

pub struct Packager {
    
}

impl Packager {
    pub async fn toolchain(&self, path: &str, addr: &str) {

        //msvc bin dir

        let content = Self::pack_compiler(path, "msvc");
        
        log::info!("sync compiler packager path: {}, size: {} KB", path, content.len() / 1024);
        Self::send_package("msvc", path, &content, addr).await;
    }

    fn pack_compiler<'a>(dir: &str, _name: &str) -> std::borrow::Cow<'a, [u8]> {
        let path = std::path::PathBuf::from(dir);

        let mut cursor = std::io::Cursor::new(Vec::new());

        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Zstd);

        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin

        log::trace!("pack path {:?}", path);
        let cl = path.clone();
        
        let _ = zip.start_file(format!("Hostx64/x64/cl.exe"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x64/cl.exe")).expect("can't find x64 cl.exe");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x64/c1.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x64/c1.dll")).expect("can't find x64 c1.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x64/c1xx.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x64/c1xx.dll")).expect("can't find x64 c1xx.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x64/c2.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x64/c2.dll")).expect("can't find x64 c2.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x64/mspdbcore.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x64/mspdbcore.dll")).expect("can't find x64 mspdbcore.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x64/tbbmalloc.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x64/tbbmalloc.dll")).expect("can't find x64 tbbmalloc.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let clui = path.clone();

        let _ = zip.start_file(format!("Hostx64/x64/1033/clui.dll"), options.clone());
        let mut file = std::fs::File::open(clui.join("Hostx64/x64/1033/clui.dll")).expect("can't find x64 clui.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/cl.exe"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x86/cl.exe")).expect("can't find x86 cl.exe");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/c1.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x86/c1.dll")).expect("can't find x86 c1.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/c1xx.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x86/c1xx.dll")).expect("can't find x86 c1xx.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/c2.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x86/c2.dll")).expect("can't find x86 c2.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/mspdbcore.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x86/mspdbcore.dll")).expect("can't find x86 mspdbcore.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/tbbmalloc.dll"), options.clone()).unwrap();
        let mut file = std::fs::File::open(cl.join("Hostx64/x86/tbbmalloc.dll")).expect("can't find x86 tbbmalloc.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let _ = zip.start_file(format!("Hostx64/x86/1033/clui.dll"), options.clone());
        let mut file = std::fs::File::open(clui.join("Hostx64/x86/1033/clui.dll")).expect("can't find x86 clui.dll");
        let _ = std::io::copy(&mut file, &mut zip);

        let content = zip.finish().unwrap();
        let file = content.to_owned().into_inner();
        let content = std::borrow::Cow::from(file);

        return content;
    }

    fn pack_dir<'a>(dir: &str, _name: &str) -> std::borrow::Cow<'a, [u8]> {
        let mut path = std::path::PathBuf::from(dir);

        let mut cursor = std::io::Cursor::new(Vec::new());

        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Zstd);

        let start = std::time::Instant::now();

        Self::zip_dir(path.as_path(), path.as_path(), &mut zip, &options);
        let content = zip.finish().unwrap();
        let file = content.to_owned().into_inner();
        let content = std::borrow::Cow::from(file);
        let elapsed = start.elapsed();

        match path.extension() {
            Some(extension) => {
                let mut ex = extension.to_str().unwrap().to_string();
                ex.push_str(".zip");
                path.set_extension(ex);
            },
            None => {
                path.set_extension("zip");
            }
        }

        log::info!("zip dir elapsed time: {:?}", elapsed);
        return content;
    }

    fn zip_dir(entry_dir: &std::path::Path, dir: &std::path::Path, zip: &mut zip::ZipWriter<&mut std::io::Cursor<Vec<u8>>>, options: &zip::write::SimpleFileOptions) {
        let mut buffer = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        let path = entry.path();
                        let name = path.strip_prefix(entry_dir).unwrap().to_str().unwrap();

                        if file_type.is_dir() {
                            zip.add_directory(name, *options).unwrap();
                            Self::zip_dir(entry_dir, &path, zip, options);
                            log::debug!("name: {:?}, path: {:?}", name, entry.path().to_str().unwrap());
                        }
                        else if file_type.is_file() {
                            zip.start_file(name, options.to_owned()).unwrap();
                            let mut file = std::fs::File::open(path).unwrap();
                            file.read_to_end(&mut buffer).unwrap();
                            zip.write_all(&buffer[..]).unwrap();
                            buffer.clear();
                        }
                    }
                    else {

                    }
                }
            }
        }
    }

    async fn send_package<'a>(name: &str, filename: &str, content: &std::borrow::Cow<'a, [u8]>, addr: &str) {

        let path = filename.to_owned() + ".zip";

        let mut sender = crate::communicate::package::Sender::new(addr, None);
        let args = crate::communicate::package::ArchiveArgs {
            file_type: crate::communicate::package::FileType::ToolChain,
            name: name.to_owned(),
            path: path,
            content: content.to_owned(),
        };
        let args: super::package::SenderType<'_> = crate::communicate::package::SenderType::Archive(args);
        sender.dist(args).await;
    }

    //do not must
    async fn windows_kits(&mut self, _kits: &str) {

    }

    pub async fn file<'a>(&self, path: &str, content: &std::borrow::Cow<'a, [u8]>, addr: &str) {

        let mut sender = crate::communicate::package::Sender::new(addr, None);

        let args = crate::communicate::package::ArchiveArgs {
            file_type: crate::communicate::package::FileType::Unknown,
            name: "precompiledsourcefile".to_string(),
            path: path.to_owned(),
            content: content.to_owned(),
        };

        let args = crate::communicate::package::SenderType::Archive(args);
        sender.dist(args).await;
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    //cargo test --package crew --tests pack_tool -- --show-output
    fn pack_tool() {
        println!("test pack msvc dir");
        //14.39.33519
        //14.37.32822
        let env = crate::platform::windows::WindowsCompilerEnv::default();
        let content = Packager::pack_compiler(env.compiler_path.to_str().unwrap(), "msvc");
        
        let cursor = std::io::Cursor::new(content);
        let zip_archive = zip::ZipArchive::new(cursor).unwrap();

        let path = tools::utils::access_working_path("").unwrap();
        println!("unzip path: {:?}", path);
        
        let expect_packages = Vec::from(["Hostx64/x64/cl.exe", "Hostx64/x64/1033/clui.dll", "Hostx64/x86/cl.exe", "Hostx64/x86/1033/clui.dll"]);
        let packages = zip_archive.file_names().collect::<Vec<&str>>();

        assert!(expect_packages == packages);
        
        let mut zip_ = zip_archive.clone();
        for name in packages {
            
            let file = zip_.by_name(name).unwrap();
            let size = file.size();
            println!("zip archive name: {}, size {}", name, size);
            assert!(size > 10000);
        }

        //let path = common::utils::get_working_path("".to_string()).unwrap();
        //let mut zip__ = zip_archive.clone();
        //zip__.extract(path).unwrap();

    }
}