
extern crate toml;

#[derive(serde_derive::Deserialize, serde_derive::Serialize, Debug)]
pub struct ConfigurationInfo {
    pub coordinator_addr: String,
    pub workers_addr: Vec<String>,
    pub redis_addr: String,
}

impl ConfigurationInfo {
    pub fn init() -> ConfigurationInfo {
        match fetch_local_config() {
            Some(value) => {
                return value;
            },
            None => {
                println!("can't load configuration file, use single tool.");
                let value = ConfigurationInfo {
                    coordinator_addr: String::from("127.0.0.1"),
                    workers_addr: vec![String::from("127.0.0.1")],
                    redis_addr: String::from("")
                };
                return value;
            },
        } 
    }    
}

fn fetch_local_config() -> Option<ConfigurationInfo> {

    let path = std::env::current_dir().unwrap();
    let config_file = path.join("config.toml");
    log::debug!("config file path: {:?}", config_file);
    match std::fs::read(config_file) {
       Ok(contents) => {
            let config: ConfigurationInfo = toml::from_slice(&contents).unwrap();
            return Some(config);
       },
       Err(error) => {
        println!("load local configuration file failed {:?}.use default configuration.", error);
        return None;
       },
    }
}