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

    fn post(&self, route: &str, input: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let base = reqwest::Url::parse("http://127.0.0.1:9302/").unwrap();
        let url = base.join(&route).unwrap();
        let response = self.client.post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&input)
            .send();
        return response;
    }

    fn post_file(&self, route: &str, path: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let file = std::fs::read(path).unwrap();
        let part = reqwest::blocking::multipart::Part::bytes(std::borrow::Cow::from(file)).file_name(path.to_owned());
        let form = reqwest::blocking::multipart::Form::new().part("file", part);
        let base = reqwest::Url::parse("http://127.0.0.1:9302/").unwrap();
        let url = base.join(route).unwrap();
        let response = self.client.post(url)
            .multipart(form)
            .header(reqwest::header::CONTENT_TYPE, "multipart/form-data")
            .send()
            .unwrap();
        return Result::Ok(response);
    }

    pub fn dist_file_sync(&self, route: &str, path: &str) {
        let _ = self.post_file(route, path);
    }


    pub fn dist_file_pre_sync(&self, route: &str, params: &str) -> Result<reqwest::blocking::Response, reqwest::Error> {
        return self.post(route, params);
    }

    pub fn dist_tool_chain_per_sync(&self, compiler_path: &str) -> crate::compiler::compiler::PreSyncFile {

        let sync_info = crate::compiler::compiler::PreSyncFile {
            file_kind: std::ffi::OsString::from("toochain"),
            file_path: std::ffi::OsString::from(compiler_path),
            file_name: std::ffi::OsString::from("cl.exe"),
            digest: std::ffi::OsString::new(),
            is_exists: false,
        };
        let sync_info = serde_json::json!(sync_info).to_string();
        let response = self.dist_file_pre_sync("presyncfile", &sync_info).unwrap();
        let value: crate::compiler::compiler::PreSyncFile = response.json().unwrap();

        return value;
    }

}
