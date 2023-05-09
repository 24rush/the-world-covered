extern crate redis;

#[cfg_attr(feature = "json", tokio::main)]

pub mod redis_db {
    use redis::Commands;
    use redis::JsonCommands;
    use redis::RedisError;

    const REDIS_DB_URL: &str = "redis://127.0.0.1:6379";

    pub struct RedisConnection {
        client: redis::Client,        
    }

    impl RedisConnection {
        pub fn new() -> Self {            
            RedisConnection {
                client: redis::Client::open(REDIS_DB_URL).expect("Opening DB URL failed")                
            }
        }

        pub fn get<K, SK, RV>(&self, key: K, sub_key: SK) -> Result<RV, RedisError>
        where
            K: redis::ToRedisArgs,
            SK: redis::ToRedisArgs,
            RV: redis::FromRedisValue,
        {
            self.connection().hget(key, sub_key)
        }

        pub fn set_set<RV>(&self, key: &str, value: &str) -> Result<RV, RedisError>
        where
            RV: redis::FromRedisValue,
        {
            self.connection()
                .sadd::<&str, &str, RV>(key, value)
        }

        pub fn json_get<RV>(&self, key: &String, sub_key: &str) -> Result<RV, RedisError>
        where
            RV: redis::FromRedisValue,
        {
            self.connection().json_get(key, sub_key)
        }

        pub fn json_set<V, RV>(&self, key: &str, sub_key: &str, value: &V) -> Result<RV, RedisError>
        where
            V: serde::Serialize,
            RV: redis::FromRedisValue,
        {
            self.connection()
                .json_set::<&str, &str, V, RV>(key, sub_key, value)
        }

        pub fn exists(&self, id: &String) -> bool {
            self.connection().exists(id).unwrap_or(false)
        }

        pub fn connection(&self) -> redis::Connection {
            self.client.get_connection().expect("Connection failed")
        }
    }
}
