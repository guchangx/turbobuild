#[derive(Clone)]
pub struct WorkingParameters {
    pub storage: std::sync::Arc<dyn crate::cache::cache::Storage>,
    pub dist: crate::dist::Dist,
    pub runtime: std::sync::Arc<tokio::runtime::Handle>,
}

fn init_tokio_runtime() -> std::sync::Arc<tokio::runtime::Handle> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    return std::sync::Arc::new(runtime.handle().clone());
}

impl Default for WorkingParameters {
    fn default() -> Self {
        let storage = crate::cache::disk::Disk::new();
        WorkingParameters {
            storage: std::sync::Arc::new(storage),
            dist: crate::dist::Dist::init(),
            runtime: init_tokio_runtime(),
        }
    }
}

impl WorkingParameters {
    pub fn init() -> Self {
        let redis = crate::cache::redis::RedisCache::new("redis://10.224.201.61/");
        let parameters = WorkingParameters {
            storage: std::sync::Arc::new(redis),
            dist: crate::dist::Dist::init(),
            runtime: init_tokio_runtime(),
        };
        return parameters;
    }
}
