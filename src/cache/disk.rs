
pub struct Disk {
    path: String
}

impl Disk {
    pub fn new() -> Disk {
        Disk { path:"".to_string() }
    }
}

#[async_trait]
impl super::cache::Storage for Disk {

    async fn exits(&self, key: &str) -> bool
    {
        return true;
    }

    async fn get(&self, key: &str) -> anyhow::Result<super::cache::Cache>
    {
        return anyhow::Ok(super::cache::Cache::Miss);
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> anyhow::Result<std::time::Duration>
    {
        let start = std::time::Instant::now();
        return anyhow::Ok(start.elapsed())
    }
}