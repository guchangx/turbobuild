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
}

pub fn _request_dist_compile() {
    
}