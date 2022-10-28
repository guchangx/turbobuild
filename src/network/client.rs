extern crate reqwest;

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

    fn post(&self, _route: String) {

        let _response = self.client.post("http://127.0.0.1:9302/requestcompile");
    }

    pub fn _request_compile(&self) {
        self.post("requestcompile".to_string())
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

    pub fn dist_file_sync(&self, path: &str) {
        let _ = self.post_file("syncfile", path);
    }
}


pub fn _request_dist_compile() {
    
}