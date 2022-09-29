

pub enum Cache {
    /// Result was found in cache.
    Hit(Vec<u8>),
    /// Result was not found in cache.
    Miss,
    /// Cache entry should be ignored, force compilation.
    _Recache,
}

#[async_trait]
pub trait Storage: core::marker::Send + core::marker::Sync + 'static {

    async fn exits(&self, key: &str) -> bool;
    async fn get(&self, key: &str) -> anyhow::Result<Cache>;
    async fn set(&self, key: &str, value: Vec<u8>) -> anyhow::Result<std::time::Duration>;
    //async fn current_size(&self) -> anyhow::Result<Option<u64>>;
    //async fn max_size(&self) -> anyhow::Result<Option<u64>>;

}