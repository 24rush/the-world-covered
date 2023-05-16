use chrono::{DateTime, NaiveDateTime, Utc};

pub (crate) mod logging;
pub (crate) mod time;

pub struct DateTimeUtils {}

impl DateTimeUtils {
    pub fn timestamp_to_str(timestamp: i64) -> String {
        let naive = NaiveDateTime::from_timestamp_opt(timestamp, 0).unwrap();

        // Create a normal DateTime from the NaiveDateTime
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

        // Format the datetime how you want
        let newdate = datetime.format("%Y-%m-%d %H:%M:%S");

        newdate.to_string()
    }

    pub fn zulu2ts(zulu_datetime: &str) -> i64 {
        NaiveDateTime::parse_from_str(zulu_datetime, "%Y-%m-%dT%H:%M:%SZ")
            .unwrap()
            .timestamp()
    }
}
