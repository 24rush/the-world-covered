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
                client: redis::Client::open(REDIS_DB_URL).expect("Opening DB URL failed"),
            }
        }

        // get string value
        pub fn gets(&self, key: &String, sub_key: &str) -> Result<String, RedisError> {
            self.get::<&String, &str, String>(key, sub_key)
        }

        // get numeric value
        pub fn getn(&self, key: &String, sub_key: &str) -> Result<f64, RedisError> {
            self.get::<&String, &str, f64>(key, sub_key)
        }

        pub fn get<K, SK, RV>(&self, key: K, sub_key: SK) -> Result<RV, RedisError>
        where
            K: redis::ToRedisArgs,
            SK: redis::ToRedisArgs,
            RV: redis::FromRedisValue,
        {
            self.connection().hget(key, sub_key)
        }

        pub fn set(&self, key: &str, sub_key: &str, value: &str) {
            self.connection()
                .hset::<&str, &str, &str, String>(key, sub_key, value)
                .unwrap();
        }

        pub fn json_get<RV>(&self, key: &String, sub_key: &str) -> Result<RV, RedisError>
        where
            RV: redis::FromRedisValue,
        {
            self.connection().json_get(key, sub_key)
        }

        pub fn json_set<V>(&self, key: &str, sub_key: &str, value: &V)
        where
            V: serde::Serialize,
        {
            self.connection()
                .json_set::<&str, &str, V, bool>(key, sub_key, value)
                .unwrap();
        }

        fn connection(&self) -> redis::Connection {
            self.client.get_connection().expect("Connection failed")
        }
    }
}
