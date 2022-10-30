#[derive(Clone)]
pub struct WorkingParameters {
    pub storage: std::sync::Arc<dyn crate::cache::cache::Storage>,
    pub dist: crate::dist::Dist,
    pub compiler_env: crate::platform::windows::WindowsCompilerEnv,
    pub network_client: crate::network::client::NetworkClient,
}

impl WorkingParameters {
    pub fn init() -> Self {
        let redis = crate::cache::redis::RedisCache::new("redis://10.224.201.61/");
        let parameters = WorkingParameters {
            storage: std::sync::Arc::new(redis),
            dist: crate::dist::Dist::init(),
            compiler_env: crate::platform::windows::WindowsCompilerEnv::default(),
            network_client: crate::network::client::NetworkClient::new(),
        };
        return parameters;
    }
}
