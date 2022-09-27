use lazy_static::__Deref;


#[derive(Clone)]
pub struct WorkingParameters {
    pub storage: std::sync::Arc<std::sync::Mutex<dyn crate::cache::cache::Storage>>,
    pub dist: crate::dist::Dist,
}

impl Default for WorkingParameters {
    fn default() -> Self {
        let storage = crate::cache::disk::Disk::new();
        WorkingParameters {
            storage: std::sync::Arc::new(std::sync::Mutex::new(storage)),
            dist: crate::dist::Dist::init(),
        }
    }
}

impl WorkingParameters {
    pub async fn init() -> Self {
        let redis = crate::cache::redis::RedisCache::new("redis://10.224.201.61/").await;

        let parameters = WorkingParameters {
            storage: std::sync::Arc::new(std::sync::Mutex::new(redis)),
            dist: crate::dist::Dist::init(),
        };
        return parameters;
    }
    pub fn set(&mut self, working_params: crate::buildturbo::WorkingParameters) {
        self.dist = working_params.dist;
        self.storage = working_params.storage;
    }
}
