extern crate redis;

#[cfg_attr(feature = "json", tokio::main)]

trait DatabaseConnection<ErrType> {
    fn new(&self);

    fn json_get(&self, key: &str, sub_key: &str) -> Result<String, ErrType>; 
    fn json_set(&self, key: &str, sub_key: &str, value: &String) -> Result<bool, ErrType>;
    fn exists(&self, id: &String) -> bool;
}

pub mod redis_db {
    use redis::Commands;
    use redis::JsonCommands;
    use redis::RedisError;

    use crate::database::DatabaseConnection;

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

        pub fn connection(&self) -> redis::Connection {
            self.client.get_connection().expect("Connection failed")
        }
    }

    impl DatabaseConnection<RedisError> for RedisConnection {
        fn new(&self) {}

        fn json_get(&self, key: &str, sub_key: &str) -> Result<String, RedisError> {
            self.connection().json_get(key, sub_key)
        }

        fn json_set(&self, key: &str, sub_key: &str, value: &String) -> Result<bool, RedisError> {
            self.connection()
                .json_set::<&str, &str, &String, bool>(key, sub_key, &value)
        }

        fn exists(&self, id: &String) -> bool {
            self.connection().exists(id).unwrap_or(false)
        }
    }
}
