
use redis::{self, AsyncCommands};

pub struct RedisCache {
    connection: redis::aio::Connection,
}

impl RedisCache {
    pub async fn new(url: &str) -> RedisCache {
        let client = redis::Client::open(url);
        match client {
            Ok(clinet) => {
                let connection = clinet.get_async_connection().await;
                match connection {
                    
                    Ok(connection) => {
                        let cache = 
                        RedisCache {
                            connection,
                        };
                        return cache;
                    },
                    Err(error) => {
                        panic!("redis client aysnc connect failed.error code: {:?}", error);
                    },
                }
            }
            Err(error) => {
                panic!("redis client opne url failed.error code: {:?}", error);
            },
        }
    }

    async fn get_obj(&mut self, key: &str) -> Result<Vec<u8>, &'static str> {
        let result: Result<Vec<u8>, redis::RedisError> = self.connection.get(key).await;
        match result {
            Result::Ok(value) => {
                println!("redis get len: {:?}", value.len());
                if value.is_empty() {
                    return Result::Err("get redis value is mepty.");
                }
                else {
                    return Result::Ok(value);
                }
            },
            Result::Err(error) => {
                println!("redis get value failed: {:?}", error);
                return Result::Err("get redis value failed.");
            },
        }
    }
    async fn set_obj(&mut self, key: &str, value: Vec<u8>) -> anyhow::Result<std::time::Duration> {
        let start = std::time::Instant::now();
        let result: Result<String, redis::RedisError> = self.connection.set(key, value).await;
        match result {
            Result::Ok(result) => {
            },
            Result::Err(error) => {
                panic!("set redis value failed, error code :{:?}", error);
            }
        }
        anyhow::Ok(start.elapsed())
    }
    async fn exists(&mut self, key: &str) -> bool {
        let result: Result<u32, redis::RedisError> = self.connection.exists(key).await;
        match result {
            Result::Ok(result) => {

                return true;
            }
            Result::Err(error) => {

                return false;
            }
        }
    }
}

#[async_trait]
impl super::cache::Storage for RedisCache {

    async fn exits(&mut self , key: &str) -> bool {
        return self.exists(key).await;
    }

    async fn get(&mut self, key: &str) -> anyhow::Result<super::cache::Cache> {
        let value = self.get_obj(key).await;
        match value {
            Ok(value) => {
                return anyhow::Result::Ok(super::cache::Cache::Hit(value))
            },
            Err(error) => {
                return anyhow::Result::Ok(super::cache::Cache::Miss)
            }
        }
    }

    async fn set(&mut self, key: &str, value: Vec<u8>) -> anyhow::Result<std::time::Duration> {
        let duration = self.set_obj(key, value).await;
        match duration {
             Ok(duration) => {
                return anyhow::Result::Ok(duration)
            },
            Err(error) => {
                panic!("")
            },
        }
    }

}