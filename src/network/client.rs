extern crate reqwest;

#[derive(Clone)]
pub struct NetworkClient {
    client: reqwest::blocking::Client,
}

impl NetworkClient {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
        .timeout(core::time::Duration::from_secs(180))
        .build()
        .expect("bulid request client failed.");

        Self {
            client,
        }
    }

    fn post<T: serde::ser::Serialize + ?core::marker::Sized>(&self, route: &str, input: &T) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&input)
            .send();
        return response;
    }

    fn dist_post(&self, route: &str, msvc_compile_input: &crate::compiler::compiler::CompileInput) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&msvc_compile_input)
            .send();
        return response;
    }

    fn file_post(&self, route: &str, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let file = std::fs::read(path).unwrap();

        let part = reqwest::blocking::multipart::Part::bytes(std::borrow::Cow::from(file)).file_name(path.to_owned());
        let form = reqwest::blocking::multipart::Form::new().part("file", part);
        let base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
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
        println!("post file by zip {:?} {:?}", name, filename);
        let part = reqwest::blocking::multipart::Part::bytes(filecontent.to_vec()).file_name(filename.to_owned());
        let form = reqwest::blocking::multipart::Form::new().part(name.to_owned(), part);
        let base = reqwest::Url::parse("http://10.140.216.142:9302/").unwrap();
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
                    println!("pre sync file post failed: {:?}", error);
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
            match myself.dist_file_pre_sync("/presyncfile", &sync_info) {
                Ok(response) => {
                    let value = response.json::<crate::compiler::compiler::SyncData>().unwrap();
                    return value;
                },
                Err(error) => {
                    println!("pre sync file post failed: {:?}", error);
                    return crate::compiler::compiler::SyncData::default()
                },
            }
        }).join().unwrap();
        println!("response: {:?}", response);
        return response;
    }

    pub fn dist_request_compile(&self, msvc_compile_input: &crate::compiler::compiler::CompileInput) -> Result<reqwest::blocking::Response, reqwest::Error> {
        println!("dist compile post");
        let myself = self.to_owned();
        let input = msvc_compile_input.to_owned();
        let response = std::thread::spawn(move || {
            return myself.dist_post("dist/requestcompile", &input)
        }).join().unwrap();
        return response;
    }

}
