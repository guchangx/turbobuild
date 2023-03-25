
use redis::{self, AsyncCommands};


pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    pub fn new(url: &str) -> RedisCache {
        let redis_addr: String;
        if url.starts_with("redis://") {
            redis_addr = url.to_string();
        }
        else {
            let mut redis_url = reqwest::Url::parse("redis://").unwrap();
            let result = redis_url.set_host(Some(url));
            if result.is_err() {
                log::warn!("redis addr is error.");
            }
            redis_addr = redis_url.to_string();
        }
        //"redis://10.224.201.61/"
        let client = redis::Client::open(redis_addr);
        match client {
            Ok(client) => {
                let redis = 
                        RedisCache {
                            client,
                        };
                        return redis;
            }
            Err(error) => {
                panic!("redis client open url failed. error code: {:?} redis: {:?}", error, url);
            },
        }
    }
    
    pub fn test(&self) -> bool {
        match self.client.get_connection_with_timeout(std::time::Duration::from_secs(2)) {
            Ok(mut con) => {
                let result: Result<String, redis::RedisError> = redis::cmd("PING").query(&mut con);
                match result {
                    Ok(value) => {
                        if value == "PONG" {
                            return true;
                        }
                        else {
                            return false;
                        }
                    },
                    Err(error)=> {
                        log::warn!("redis ping query error: {:?}", error);
                        return false;
                    },
                }
            },
            Err(error) => {
                log::warn!("redis get connection error: {:?}", error);
                return false;
            }
        }
    }

    async fn connect(&self) -> anyhow::Result<redis::aio::Connection> {
        let connection = self.client.get_async_connection().await?;
        return anyhow::Ok(connection);
    }

    async fn get_obj(&self, key: &str) -> anyhow::Result<Vec<u8>> {
        let mut connection = self.connect().await?;
        let result: Result<Vec<u8>, redis::RedisError> = connection.get(key).await;
        match result {
            Result::Ok(value) => {
                if value.is_empty() {
                    return anyhow::Result::Err(anyhow::Error::msg("get from redis value is empty"));
                }
                else {
                    return Result::Ok(value);
                }
            },
            Result::Err(error) => {
                println!("redis get value failed: {:?}", error);
                return anyhow::Result::Err(anyhow::Error::from(error));
            },
        }
    }
    async fn set_obj(&self, key: &str, value: Vec<u8>) -> anyhow::Result<std::time::Duration> {
        let mut connection = self.connect().await?;
        let start = std::time::Instant::now();
        let result: Result<String, redis::RedisError> = connection.set(key, value).await;
        match result {
            Result::Ok(result) => {
                if result.contains("OK") {

                } else {
                    println!("set object to redis result failed.");
                }
            },
            Result::Err(error) => {
                return anyhow::Result::Err(anyhow::Error::from(error));
            }
        }
        anyhow::Ok(start.elapsed())
    }
    async fn exists(&self, key: &str) -> bool {
        let connection = self.connect().await;
        match connection {
            Ok(mut connection) => {
                let result: Result<u32, redis::RedisError> = connection.exists(key).await;
                    match result {
                        Result::Ok(result) => {
                            if result == 1 {
                                return true;
                            }
                            else {
                                return false;
                            }
                        }
                        Result::Err(error) => {
                            println!("exists key failed. error code: {:?}", error);
                            return false;
                        }
                    }
            },
            Err(error) => {
                println!("exists key conenct failed. error code: {:?}", error);
                return false;
            },
        }
    }
}

#[async_trait]
impl super::cache::Storage for RedisCache {

    async fn exits(&self , key: &str) -> bool {
        return self.exists(key).await;
    }

    async fn get(&self, key: &str) -> anyhow::Result<super::cache::Cache> {
        let value = self.get_obj(key).await;
        match value {
            Ok(value) => {
                return anyhow::Result::Ok(super::cache::Cache::Hit(value))
            },
            Err(error) => {
                return anyhow::Result::Err(error);
            }
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> anyhow::Result<std::time::Duration> {
        let duration = self.set_obj(key, value).await;
        match duration {
             Ok(duration) => {
                return anyhow::Result::Ok(duration)
            },
            Err(error) => {
               return anyhow::Result::Err(error);
            },
        }
    }

}