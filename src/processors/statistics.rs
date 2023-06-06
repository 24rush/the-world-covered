use std::collections::HashMap;

use chrono::{Datelike, NaiveDateTime};
use mongodb::bson::doc;

use crate::{
    data_types::{gc::stats::YearlyStats, strava::athlete::AthleteId},
    logln,
    util::facilities::{Facilities, Required},
};

pub struct Statistics<'a> {
    dependencies: &'a mut Facilities<'a>,
}

impl<'a> Statistics<'a> {
    const CC: &str = "Statistics";

    pub fn new(dependencies: &'a mut Facilities<'a>) -> Self {
        dependencies.check(vec![Required::GcDB, Required::StravaDB]);

        Self { dependencies }
    }
    
    pub async fn collect_yearly_stats(&self, ath_id: AthleteId) -> Vec<YearlyStats> {
        let mut yearly_stats: HashMap<i32, YearlyStats> = HashMap::new();

        // Get all matched activities and fill in all the efforts
        let activities = self
            .dependencies
            .strava_db()
            .query_activities(vec![
                doc! {"$match": {"athlete.id": ath_id}},
                doc! {"$sort": {"start_date_local": 1}},
            ])
            .await;

        let mut counter = 0;
        for activity in &activities {
            logln!("{}", activity.start_date_local);

            let naive_datetime =
                NaiveDateTime::parse_from_str(&activity.start_date_local, "%Y-%m-%dT%H:%M:%SZ")
                    .unwrap();

            let year = naive_datetime.year();

            let mut year_stats = yearly_stats.entry(year).or_default();
            year_stats.year = year as u32;

            if counter > 5 {
                break;
            }

            counter += 1;
        }

        yearly_stats.values().into_iter().cloned().collect()
    }
}
