extern crate reqwest;

#[derive(Clone)]
pub struct NetworkClient {
    client: reqwest::blocking::Client,
    workers_addr:Vec<String>,
}

impl NetworkClient {
    pub fn new(workers_addr: &Vec<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
        .timeout(core::time::Duration::from_secs(180))
        .build()
        .expect("bulid request client failed.");

        Self {
            client,
            workers_addr: workers_addr.to_owned(),
        }
    }

    fn post<T: serde::ser::Serialize + ?core::marker::Sized>(&self, route: &str, input: &T) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let mut base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        match self.workers_addr.get(0) {
            Some(addr) => {
                let _ = base.set_host(Some(&addr));
            },
            None => {
                log::debug!("Don't have workers");
            },
        }
        
        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&input)
            .send();
        return response;
    }

    fn dist_post(&self, route: &str, msvc_compile_input: &crate::compiler::compiler::CompileInput) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let mut base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        match self.workers_addr.get(0) {
            Some(addr) => {
                let _ = base.set_host(Some(&addr));
            },
            None => {
                log::debug!("Don't have workers");
            },
        }
        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&msvc_compile_input)
            .send();
        return response;
    }

    fn dist_mutlipart_post(&self, route: &str, msvc_compile_input: &crate::compiler::compiler::CompileInput, precompiled_source: &crate::compiler::compiler::PrecompiledSource) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let mut base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        match self.workers_addr.get(0) {
            Some(addr) => {
                let _ = base.set_host(Some(&addr));
            },
            None => {
                log::debug!("Don't have workers");
            },
        }
        let precompiled_source = precompiled_source.to_owned();
        let mut form = reqwest::blocking::multipart::Form::new();
        if let Some(contents) = precompiled_source.preprocessed_source_contents {
            let mut part = reqwest::blocking::multipart::Part::bytes(std::borrow::Cow::from(contents));
            if let Ok(path) = precompiled_source.preprocessed_source_path.into_string() {
                part = part.file_name(path);
            }
            form = form.part("precompiled_source", part);
        }
        else
        {
            log::debug!("precompiled source file is mepty.")
        }

        if let Ok(input) = serde_json::to_string(msvc_compile_input) {
            let part = reqwest::blocking::multipart::Part::bytes(input.as_bytes().to_owned());
            form = form.part("compile_input", part);
        }
        else {
            log::debug!("Serialize compile input struct into string failed.")
        }

        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
            .multipart(form)
            .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
            .send();
        return response;
    }

    fn file_post(&self, route: &str, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let file = std::fs::read(path).unwrap();

        let part = reqwest::blocking::multipart::Part::bytes(std::borrow::Cow::from(file)).file_name(path.to_owned());
        let form = reqwest::blocking::multipart::Form::new().part("file", part);
        let mut base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        match self.workers_addr.get(0){
            Some(addr) => {
                let _ = base.set_host(Some(&addr));
            },
            None => {
                log::debug!("Don't have workers");
            },
        }
        let url = base.join(route).unwrap();
        let response = self.client.post(url)
            .multipart(form)
            .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
            .send();
        match response {
            Ok(res) => {
                return Result::Ok(res);
            },
            Err(error) => {
                println!("post file failed: {:?}", error);
                return Result::Err(error);
            }
        }
    }

    fn file_post_by_zip(&self, route: &str, name: &str, filename: &str, filecontent: &std::borrow::Cow<[u8]>) -> Result<reqwest::blocking::Response, reqwest::Error> {
        log::debug!("post file by zip {:?} {:?}", name, filename);
        let part = reqwest::blocking::multipart::Part::bytes(filecontent.to_vec()).file_name(filename.to_owned());
        let form = reqwest::blocking::multipart::Form::new().part(name.to_owned(), part);
        let mut base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        match self.workers_addr.get(0){
            Some(addr) => {
                let _ = base.set_host(Some(&addr));
            },
            None => {
                log::debug!("Don't have workers");
            },
        }
        let url = base.join(route).unwrap();
        let response = self.client.post(url)
            .multipart(form)
            .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
            .send();
        match response {
            Ok(res) => {
                return Result::Ok(res);
            },
            Err(error) => {
                println!("post file failed: {:?}", error);
                return Result::Err(error);
            }
        }
    }

    pub fn dist_file_sync(&self, route: &str, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let route = route.to_owned();
        let path = path.to_owned();
        let myself = self.to_owned();
        let response = std::thread::spawn(move || {
            return myself.file_post(&route, &path);
        }).join().unwrap();

        return response;
    }

    pub fn dist_zip_sync(&self, route: &str, name: &str, filename: &str, filecontent: &std::borrow::Cow<[u8]>) -> crate::compiler::compiler::SyncData {
        
        let myself = self.to_owned();
        let route = route.to_owned();
        let name = name.to_owned();
        let filename = filename.to_owned();
        let filecontent = filecontent.to_owned().into_owned();
        let response = std::thread::spawn(move || {
            match myself.file_post_by_zip(&route, &name, &filename, &std::borrow::Cow::from(filecontent)) {
                Ok(response) => {
                    let value = response.json::<crate::compiler::compiler::SyncData>().unwrap();
                    return value;
                },
                Err(error) => {
                    println!("pre sync file failed: {:?}", error);
                    return crate::compiler::compiler::SyncData::default()
                },
            }
        }).join().unwrap();
        return response;
    }


    pub fn dist_file_pre_sync<T: serde::ser::Serialize + ?core::marker::Sized>(&self, route: &str, params: &T) -> Result<reqwest::blocking::Response, reqwest::Error> {
        return self.post(route, params);
    }

    pub fn dist_kits_and_tool_pre_sync(&self, sync_info: &crate::compiler::compiler::SyncData) -> crate::compiler::compiler::SyncData {

        let myself = self.to_owned();
        let sync_info = sync_info.to_owned();
        let response = std::thread::spawn(move || {
            match myself.dist_file_pre_sync("dist/presyncfile", &sync_info) {
                Ok(response) => {
                    let value = response.json::<crate::compiler::compiler::SyncData>().unwrap();
                    return value;
                },
                Err(error) => {
                    println!("pre sync kits and toolchain failed: {:?}", error);
                    return crate::compiler::compiler::SyncData::default()
                },
            }
        }).join().unwrap();
        return response;
    }

    pub fn dist_request_compile_with_source_and_include(&self, msvc_compile_input: &crate::compiler::compiler::CompileInput) -> crate::compiler::compiler::CompileOutput {
        let myself = self.to_owned();
        let input = msvc_compile_input.to_owned();
        let response = std::thread::spawn(move || {
            match myself.dist_post("dist/requestcompile/sourcefile", &input) {
                Ok(response) => {
                    let value = response.json::<crate::compiler::compiler::CompileOutput>().unwrap();
                    return value;
                }, 
                Err(_) => {
                    return crate::compiler::compiler::CompileOutput::default();
                }
            }
        }).join().unwrap();
        return response;
    }

    pub fn dist_request_compile_with_precompiled_source(&self, msvc_compile_input: &crate::compiler::compiler::CompileInput, precompiled_source: &crate::compiler::compiler::PrecompiledSource) -> crate::compiler::compiler::CompileOutput {
        let myself = self.to_owned();
        let input = msvc_compile_input.to_owned();
        let precompiled = precompiled_source.to_owned();
        let response = std::thread::spawn(move || {
            match myself.dist_mutlipart_post("dist/requestcompile/precompiled", &input, &precompiled) {
                Ok(response) => {
                    if response.status() == axum::http::StatusCode::OK {
                        let headers = response.headers();
                        if let Some(output) = headers.get("compile-output") {
                            let output: crate::compiler::compiler::CompileOutput = serde_json::from_slice(output.as_bytes())
                                        .expect("Deserialize from JSON into CompileOutput failed.");
                            if let Some(results) = headers.get("compile-result-catalog") {
                                let results: Vec<Vec<(std::ffi::OsString, usize)>> = serde_json::from_slice(results.as_bytes())
                                        .expect("Deserialize from JSON into <<Path, content>> failed.");

                                if let Ok(contents) = response.bytes() {
                                    if !contents.is_empty() && !results.is_empty() {
                                        let mut contents = contents.to_vec();

                                        for result in results {
                                            for (path, size) in result {
                                                if !contents.is_empty() {
                                                    let (current_content, other_content) = contents.split_at(size);
                                                    match std::fs::write(path.clone(), current_content) {
                                                        Ok(_) => {
                                                            log::trace!("sync results, {:?}.", path);
                                                        },
                                                        Err(error) => {
                                                            log::warn!("sync results, wriet {:?} failed {:?} .", path, error);
                                                        },
                                                    }
                                                    contents = other_content.to_vec();
                                                }
                                            }
                                        }        
                                    }
                                }
                            }
                            return output;
                        }
                    }
                    return crate::compiler::compiler::CompileOutput::default();
                }, 
                Err(_) => {
                    return crate::compiler::compiler::CompileOutput::default();
                }
            }
        }).join().unwrap();
        return response;
    }
}
