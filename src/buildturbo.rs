
#[derive(Clone)]
pub struct WorkingParameters {
    pub storage: std::sync::Arc<dyn crate::cache::cache::Storage>,
    pub dist: crate::dist::Dist,
    pub compiler_env: crate::platform::windows::WindowsCompilerEnv,
    pub network_client: crate::network::client::NetworkClient,
}

impl WorkingParameters {
    pub fn init(config: &crate::config::ConfigurationInfo) -> Self {
        let redis = crate::cache::redis::RedisCache::new(&config.redis_addr);
        let parameters = WorkingParameters {
            storage: std::sync::Arc::new(redis),
            dist: crate::dist::Dist::init(), 
            compiler_env: crate::platform::windows::WindowsCompilerEnv::default(),
            network_client: crate::network::client::NetworkClient::new(&config.workers_addr),
        };
        return parameters;
    }
}
