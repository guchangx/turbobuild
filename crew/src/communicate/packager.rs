use std::io::{Read, Write};

#[derive(Default, Clone)]

pub struct Packager {

}

impl Packager {
    pub async fn toolchain(&self, path: &str) {

        println!("path: {:?}", path);
        
        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\
        //msvc bin dir

        let content = Self::pack_dir(path, "msvc");
        
        Self::send_package("msvc", path, &content).await;

        let mut path = std::path::PathBuf::from(path);
        //msvc include
        for _ in 0..3 {
            path.pop();
        }
        let include = path.join("include");

        if include.is_dir() {
            println!("sync dir: {:?}", path);

            let content = Self::pack_dir(include.to_str().unwrap(), "msvc");
            Self::send_package("msvc", path.to_str().unwrap(), &content).await;
        }
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

    async fn send_package<'a>(name: &str, filename: &str, content: &std::borrow::Cow<'a, [u8]>) {

        let mut sender = crate::communicate::package::FileSender::new();
        let args = crate::communicate::package::ArchiveArgs {
            file_type: crate::communicate::package::FileType::ToolChain,
            name: name.to_owned(),
            path: filename.to_owned(),
            content: content.to_owned(),
        };
        let args = crate::communicate::package::SenderType::Archive(args);
        sender.send(args).await;
    }

    //do not must
    async fn windows_kits(&mut self, _kits: &str) {

    }

    pub async fn file<'a>(&self, path: &str, content: &std::borrow::Cow<'a, [u8]>) {
        let mut sender = crate::communicate::package::FileSender::new();

        let args = crate::communicate::package::ArchiveArgs {
            file_type: crate::communicate::package::FileType::PrecompileedFile,
            name: "precompiledsourcefile".to_string(),
            path: path.to_owned(),
            content: content.to_owned(),
        };

        let args = crate::communicate::package::SenderType::Archive(args);
        sender.send(args).await;
    }

    pub async fn check_resource() -> Option<crate::platform::windows::WindowsCompilerEnv> {
        let mut sender = crate::communicate::package::FileSender::new();
        let env = sender.check_resource().await;
        return env;
    }
}