use strava::Api;

mod database;
mod strava;

fn main() {
    let redis_con = database::redis_db::RedisConnection::new();
    let s: Result<String, redis::RedisError> = redis_con.get("45435345", "ertert");

    let api = Api::new(&redis_con);

    if let Ok(s) = s {
        println!("{:?}", s);
    }
}
