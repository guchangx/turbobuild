
pub struct Disk {
    _path: String
}

impl Disk {
    pub fn _new() -> Disk {
        Disk { _path:"".to_string() }
    }
}

#[async_trait]
impl super::cache::Storage for Disk {

    async fn exits(&self, _key: &str) -> bool
    {
        return true;
    }

    async fn get(&self, _key: &str) -> anyhow::Result<super::cache::Cache>
    {
        return anyhow::Ok(super::cache::Cache::Miss);
    }

    async fn set(&self, _key: &str, _value: Vec<u8>) -> anyhow::Result<std::time::Duration>
    {
        let start = std::time::Instant::now();
        return anyhow::Ok(start.elapsed())
    }
}